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
use crate::errors::error_in;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{
    back_error_in, back_input_in, begin_token_list_in, InputAbort, INSERTED, LEFT_BRACE_LIMIT,
    MACRO, MARK_TEXT, RIGHT_BRACE_LIMIT, TOKEN_LIST, V_TEMPLATE,
};
use crate::mem::{BACKUP_HEAD, GARBAGE, TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyBox, AnyNode, CondNode, NodePtr, SaNode};
use crate::output::{
    begin_diagnostic_in, capture_to_diagnostic_in, diagnostic_begin_capture_warning_here_in,
    end_diagnostic_in, error_here_with_diagnostic_in, print_char_in, print_cmd_chr_in,
    print_cstr_in, print_err_in, print_esc_cstr_in, print_int_in, print_scaled_in,
    show_cur_cmd_chr_in, sprint_cs_in,
};
use crate::scan::{
    fetch_box, get_avail_scc, get_next_scc, get_token_in, get_x_non_blank_in, get_x_token_in,
    scan_int_in, scc_err, store_new_token, str_toks_scc, SccAbort, END_MATCH_TOKEN, MARK_VAL,
    MATCHING, MATCH_TOKEN, MAX_CHAR_VAL, NORMAL_SCANNER, NO_EXPAND_FLAG, OTHER_TOKEN, SPACE_TOKEN,
};
use crate::state::EngineState;
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

pub(crate) fn ins_the_toks_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let tail = crate::scan::the_toks_in(st)?;
    ins_temp_head_list(st, tail)
}

/// Inserts the list at `link(temp_head)` whose tail is `tail`.
fn ins_temp_head_list(st: &mut EngineState, tail: MemPtr) -> Result<(), SccAbort> {
    st.mem.set_b32s1(GARBAGE, tail.0);
    let list = st.mem.b32s1(TEMP_HEAD);
    begin_token_list_in(st, list, INSERTED)?;
    Ok(())
}

fn pop_cond_node_in(st: &mut EngineState) {
    let p = NodePtr::<CondNode>::at(st.scanner.cond_ptr);
    (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) = p.saved_state(&st.mem);
    st.scanner.cond_ptr = p.link(&st.mem);
    p.free(&mut st.mem);
}

fn pop_fi_in(st: &mut EngineState) {
    if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr {
        crate::output::if_warning_in(st);
    }
    pop_cond_node_in(st);
}

/// Handles the `\else` or `\fi` that ended a skipped conditional branch.
fn skipped_branch_end_in(st: &mut EngineState) {
    if st.scanner.cur_chr == FI_CODE as i32 {
        pop_fi_in(st);
    } else {
        st.scanner.if_limit = FI_CODE;
    }
}

fn get_x_token_or_active_char_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_x_token_in(st)?;
    if st.scanner.cur_cmd == RELAX && st.scanner.cur_chr == NO_EXPAND_FLAG {
        st.scanner.cur_cmd = ACTIVE_CHAR;
        st.scanner.cur_chr = st.scanner.cur_tok - (CS_TOKEN_FLAG + ACTIVE_BASE);
    }
    Ok(())
}

fn get_next_unexpanded_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = NORMAL_SCANNER;
    get_next_scc(st)?;
    st.scanner.scanner_status = save_scanner_status;
    Ok(())
}

fn get_token_unexpanded_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = NORMAL_SCANNER;
    get_token_in(st)?;
    st.scanner.scanner_status = save_scanner_status;
    Ok(())
}

/// Copies the character tokens of `list` into `buffer[first..]`, returning the end index.
fn token_chars_to_buffer(st: &mut EngineState, list: MemPtr) -> Result<i32, SccAbort> {
    let mut j = st.input.first;
    let mut p = list;
    while !p.is_null() {
        if j >= st.input.max_buf_stack {
            st.input.max_buf_stack = j + 1;
            if st.input.max_buf_stack == st.input.buf_size {
                return Err(SccAbort::Input(InputAbort::Overflow {
                    what: c"buffer size".as_ptr(),
                    n: st.input.buf_size,
                }));
            }
        }
        st.input.buffer[j as usize] = p.info(&st.mem) % MAX_CHAR_VAL;
        j += 1;
        p = p.link(&st.mem);
    }
    Ok(j)
}

/// Scans the tokens of a `\csname`-like name up to the terminating control sequence.
fn scan_cs_name_tokens_in(st: &mut EngineState) -> Result<MemPtr, SccAbort> {
    let head = get_avail_scc(st)?;
    let mut tail = head;
    let saved_in_csname = st.scanner.is_in_csname;
    st.scanner.is_in_csname = true;
    loop {
        get_x_token_in(st)?;
        if st.scanner.cur_cs != 0 {
            break;
        }
        store_new_token(st, &mut tail, st.scanner.cur_tok)?;
    }
    if st.scanner.cur_cmd != END_CS_NAME {
        error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
        print_esc_cstr_in(st, c"endcsname".as_ptr());
        print_cstr_in(st, c" inserted".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help(&[
            "The control sequence marked <to be read again> should",
            r"not appear between \csname and \endcsname.",
        ]);
        scc_err(back_error_in(st)?)?;
    }
    st.scanner.is_in_csname = saved_in_csname;
    Ok(head)
}

fn trace_diagnostic_in(st: &mut EngineState, text: &std::ffi::CStr) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    print_cstr_in(st, text.as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

pub(crate) fn conditional_in(st: &mut EngineState) -> Result<(), SccAbort> {
    if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0 && st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) <= 1 {
        show_cur_cmd_chr_in(st);
    }

    let cond_node =
        NodePtr::<CondNode>::alloc(&mut st.mem).ok_or_else(crate::scan::main_mem_overflow_scc)?;
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
            get_x_token_or_active_char_in(st)?;
            let (m, n) = if st.scanner.cur_cmd > ACTIVE_CHAR || st.scanner.cur_chr > BIGGEST_USV {
                (RELAX, TOO_BIG_USV)
            } else {
                (st.scanner.cur_cmd, st.scanner.cur_chr)
            };
            get_x_token_or_active_char_in(st)?;
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
            scan_int_or_dimen_in(st, this_if)?;
            let n = st.scanner.cur_val;
            get_x_non_blank_in(st)?;
            let relation = if (OTHER_TOKEN + b'<' as i32..=OTHER_TOKEN + b'>' as i32)
                .contains(&st.scanner.cur_tok)
            {
                (st.scanner.cur_tok - OTHER_TOKEN) as u8
            } else {
                error_here_with_diagnostic_in(st, c"Missing = inserted for ".as_ptr());
                print_cmd_chr_in(st, IF_TEST, this_if as i32);
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors
                    .help(&["I was expecting to see `<', `=', or `>'. Didn't."]);
                scc_err(back_error_in(st)?)?;
                b'='
            };
            scan_int_or_dimen_in(st, this_if)?;
            let cv = st.scanner.cur_val;
            match relation {
                b'<' => n < cv,
                b'=' => n == cv,
                _ => n > cv,
            }
        }
        IF_ODD_CODE => {
            scan_int_in(st)?;
            st.scanner.cur_val % 2 != 0
        }
        IF_VMODE_CODE => st.input.cur_list.mode.abs() == VMODE,
        IF_HMODE_CODE => st.input.cur_list.mode.abs() == HMODE,
        IF_MMODE_CODE => st.input.cur_list.mode.abs() == MMODE,
        IF_INNER_CODE => st.input.cur_list.mode < 0,
        IF_VOID_CODE | IF_HBOX_CODE | IF_VBOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
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
            get_next_scc(st)?;
            let n = st.scanner.cur_cs;
            let p = st.scanner.cur_cmd;
            let q = st.scanner.cur_chr;
            get_next_scc(st)?;
            let b = if st.scanner.cur_cmd != p {
                false
            } else if st.scanner.cur_cmd < CALL {
                st.scanner.cur_chr == q
            } else {
                let mut p = st.mem.b32s1(st.scanner.cur_chr);
                let mut q = st.mem.b32s1(st.eqtb.b32s1(n));
                if p == q {
                    true
                } else {
                    while p != TEX_NULL && q != TEX_NULL {
                        if st.mem.b32s0(p) != st.mem.b32s0(q) {
                            p = TEX_NULL;
                        } else {
                            p = st.mem.b32s1(p);
                            q = st.mem.b32s1(q);
                        }
                    }
                    p == TEX_NULL && q == TEX_NULL
                }
            };
            st.scanner.scanner_status = save_scanner_status;
            b
        }
        IF_EOF_CODE => {
            crate::scan::scan_four_bit_int_or_18_in(st)?;
            if st.scanner.cur_val == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[st.scanner.cur_val as usize] == crate::consts::CLOSED
            }
        }
        IF_TRUE_CODE => true,
        IF_FALSE_CODE => false,
        IF_DEF_CODE => {
            get_next_unexpanded_in(st)?;
            st.scanner.cur_cmd != UNDEFINED_CS
        }
        IF_CS_CODE => {
            let head = scan_cs_name_tokens_in(st)?;
            let first = st.input.first;
            let m = token_chars_to_buffer(st, head.link(&st.mem))?;
            st.scanner.cur_cs = if m == first {
                NULL_CS
            } else if m == first + 1 {
                SINGLE_BASE + st.input.buffer[first as usize]
            } else {
                crate::hash::id_lookup(
                    &mut st.eqtb,
                    &mut st.strings,
                    &st.input.buffer,
                    first,
                    m - first,
                )
                .map_err(|err| SccAbort::Input(crate::scan::hash_err_abort(err)))?
            };
            crate::mem::flush_list(&mut st.mem, head);
            st.eqtb.eq_type(st.scanner.cur_cs) != UNDEFINED_CS
        }
        IF_IN_CSNAME_CODE => st.scanner.is_in_csname,
        IF_FONT_CHAR_CODE => {
            crate::scan::scan_font_ident_in(st)?;
            let f = st.scanner.cur_val;
            crate::scan::scan_usv_num_in(st)?;
            let c = st.scanner.cur_val;
            let fa = st.fonts.font_area[f as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { crate::ext::map_char_to_glyph(&st.fonts, f, c) }? > 0
            } else if st.fonts.font_bc[f as usize] as i32 <= c
                && st.fonts.font_ec[f as usize] as i32 >= c
            {
                let ec = crate::fonts::effective_char(&mut st.fonts, f, c as u16);
                let info = unsafe {
                    st.fonts.font_info[(st.fonts.char_base[f as usize] + ec) as usize].b16
                };
                info.s3 > 0
            } else {
                false
            }
        }
        IF_CASE_CODE => {
            scan_int_in(st)?;
            let mut n = st.scanner.cur_val;
            if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
                begin_diagnostic_in(st);
                diagnostic_begin_capture_warning_here_in(st);
                print_cstr_in(st, c"{case ".as_ptr());
                print_int_in(st, n);
                print_char_in(st, b'}' as i32);
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                end_diagnostic_in(st, false);
            }
            while n != 0 {
                crate::scan::pass_text_in(st)?;
                if st.scanner.cond_ptr.0 == save_cond_ptr {
                    if st.scanner.cur_chr != OR_CODE as i32 {
                        skipped_branch_end_in(st);
                        return Ok(());
                    }
                    n -= 1;
                } else if st.scanner.cur_chr == FI_CODE as i32 {
                    pop_fi_in(st);
                }
            }
            scc_err(crate::output::change_if_limit_in(
                st,
                OR_CODE as i16,
                save_cond_ptr,
            ))?;
            return Ok(());
        }
        IF_PRIMITIVE_CODE => {
            get_next_unexpanded_in(st)?;
            let m = crate::hash::prim_lookup_cs(&mut st.eqtb, &mut st.strings, st.scanner.cur_cs)
                .map_err(|err| SccAbort::Input(crate::scan::hash_err_abort(err)))?;
            st.scanner.cur_cmd != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && st.scanner.cur_cmd == st.eqtb.eq_type(PRIM_EQTB_BASE + m)
                && st.scanner.cur_chr == st.eqtb.b32s1(PRIM_EQTB_BASE + m)
        }
        _ => false,
    };

    if is_unless {
        b = !b;
    }

    if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
        trace_diagnostic_in(st, if b { c"{true}" } else { c"{false}" });
    }

    if b {
        scc_err(crate::output::change_if_limit_in(
            st,
            ELSE_CODE as i16,
            save_cond_ptr,
        ))?;
        return Ok(());
    }

    loop {
        crate::scan::pass_text_in(st)?;
        if st.scanner.cond_ptr.0 == save_cond_ptr {
            if st.scanner.cur_chr != OR_CODE as i32 {
                skipped_branch_end_in(st);
                return Ok(());
            }
            error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
            print_esc_cstr_in(st, c"or".as_ptr());
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors
                .help(&[r"I'm ignoring this; it doesn't match any \if."]);
            scc_err(error_in(st))?;
        } else if st.scanner.cur_chr == FI_CODE as i32 {
            pop_fi_in(st);
        }
    }
}

fn scan_int_or_dimen_in(st: &mut EngineState, this_if: i16) -> Result<(), SccAbort> {
    if this_if == IF_INT_CODE {
        scan_int_in(st)
    } else {
        crate::scan::scan_dimen_in(st, false, false, false)
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
fn save_scan_in(st: &mut EngineState) -> Result<SavedScan, SccAbort> {
    let saved = SavedScan {
        status: st.scanner.scanner_status,
        warning_index: st.scanner.warning_index,
        def_ref: st.scanner.def_ref.0,
        pending_str: st.strings.cur_string_start() < st.strings.pool_ptr,
    };
    if saved.pending_str {
        crate::string_pool::make_string(&mut st.strings)
            .map_err(|_| make_string_overflow_scc(st))?;
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
fn scan_text_to_string_in(st: &mut EngineState, saved: &SavedScan) -> Result<i32, SccAbort> {
    crate::scan::scan_pdf_ext_toks_in(st)?;
    if st.shipout.selector == SELECTOR_NEW_STRING {
        return Err(SccAbort::PdfError {
            t: c"tokens".as_ptr(),
            p: c"tokens_to_string() called while selector = new_string".as_ptr(),
        });
    }
    let old_selector = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let text = st.mem.b32s1(st.scanner.def_ref);
    let limit = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, text, TEX_NULL, limit);
    st.shipout.selector = old_selector;
    let s = crate::string_pool::make_string(&mut st.strings)
        .map_err(|_| make_string_overflow_scc(st))?;
    crate::mem::delete_token_ref(&mut st.mem, st.scanner.def_ref);
    restore_scan(st, saved);
    Ok(s)
}

/// Inserts the pool text from `b` produced for the file-query string `s`.
fn ins_file_query_result_in(
    st: &mut EngineState,
    saved: &SavedScan,
    s: i32,
    b: i32,
) -> Result<(), SccAbort> {
    let tail = str_toks_scc(st, b, 0)?;
    crate::string_pool::flush_str(&mut st.strings, s);
    ins_temp_head_list(st, tail)?;
    unseal_pending_str(st, saved);
    Ok(())
}

fn scan_nonnegative_int_in(
    st: &mut EngineState,
    keyword: &std::ffi::CStr,
    err: &std::ffi::CStr,
    help: &'static str,
) -> Result<i32, SccAbort> {
    st.scanner.cur_val = 0;
    if crate::scan::scan_keyword_in(st, keyword)? {
        scan_int_in(st)?;
        if st.scanner.cur_val < 0 {
            print_err_in(st, err);
            st.errors.help(&[help, "I changed this one to zero."]);
            scc_err(crate::errors::int_error_in(st, st.scanner.cur_val))?;
            st.scanner.cur_val = 0;
        }
    }
    Ok(st.scanner.cur_val)
}

/// Whether a node may be skipped when looking for a margin kern at the edge of a line.
fn is_margin_kern_filler(mem: &crate::state::MemState, node: AnyNode, par_skip: i16) -> bool {
    match node {
        AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
        AnyNode::Disc(d) => {
            d.pre_break(mem).is_null() && d.post_break(mem).is_null() && d.subtype(mem) == 0
        }
        AnyNode::Math(m) => m.width(mem).0 == 0,
        AnyNode::Kern(k) => k.width(mem).0 == 0 || k.subtype(mem) == 0,
        AnyNode::Glue(g) => g.glue_ptr(mem) == ZERO_GLUE || g.subtype(mem) == (par_skip + 1) as u16,
        AnyNode::HList(h) => {
            h.width(mem).0 == 0
                && h.height(mem).0 == 0
                && h.depth(mem).0 == 0
                && h.list_ptr(mem).is_null()
        }
        _ => false,
    }
}

/// Prints the width of the margin kern of `side` found by walking from `p` with `next`,
/// skipping fillers, or `0pt` if there is none.
fn print_margin_kern_in(
    st: &mut EngineState,
    mut p: i32,
    side: u16,
    par_skip: i16,
    next: impl Fn(&crate::state::MemState, i32) -> i32,
) {
    let width = loop {
        if p == TEX_NULL || p >= st.mem.hi_mem_min {
            break None;
        }
        match st.mem.classify(MemIdx::new(p)) {
            Ok(node) if is_margin_kern_filler(&st.mem, node, par_skip) => p = next(&st.mem, p),
            Ok(AnyNode::MarginKern(mk)) if mk.subtype(&st.mem) == side => {
                break Some(mk.width(&st.mem).0)
            }
            _ => break None,
        }
    };
    match width {
        Some(w) => print_scaled_in(st, w),
        None => print_char_in(st, '0' as i32),
    }
    print_cstr_in(st, c"pt".as_ptr());
}

pub(crate) fn conv_toks_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let c = st.scanner.cur_chr;
    let mut cat: i32 = 0;
    let mut fnt: i32 = 0;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut margin_list = TEX_NULL;

    match c {
        NUMBER_CODE | ROMAN_NUMERAL_CODE | UNIFORM_DEVIATE_CODE => scan_int_in(st)?,
        STRING_CODE | MEANING_CODE => get_token_unexpanded_in(st)?,
        FONT_NAME_CODE => crate::scan::scan_font_ident_in(st)?,
        EXPANDED_CODE => {
            let saved = save_scan_in(st)?;
            crate::scan::scan_pdf_ext_toks_in(st)?;
            st.scanner.warning_index = saved.warning_index;
            st.scanner.scanner_status = saved.status;
            let def_ref = st.scanner.def_ref;
            let list = def_ref.link(&st.mem);
            begin_token_list_in(st, list.0, INSERTED)?;
            crate::mem::free_avail(&mut st.mem, st.scanner.def_ref);
            st.scanner.def_ref.0 = saved.def_ref;
            unseal_pending_str(st, &saved);
            return Ok(());
        }
        LEFT_MARGIN_KERN_CODE | RIGHT_MARGIN_KERN_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let Some(AnyBox::HList(hbox)) = fetch_box(st, st.scanner.cur_val) else {
                return Err(SccAbort::PdfError {
                    t: c"marginkern".as_ptr(),
                    p: c"a non-empty hbox expected".as_ptr(),
                });
            };
            margin_list = hbox.list_ptr(&st.mem).0;
        }
        PDF_CREATION_DATE_CODE => {
            let b = st.strings.pool_ptr;
            crate::texmfmp::getcreationdate_in(&st.texmf, &mut st.strings);
            let tail = str_toks_scc(st, b, 0)?;
            ins_temp_head_list(st, tail)?;
        }
        PDF_FILE_MOD_DATE_CODE => {
            let saved = save_scan_in(st)?;
            let s = scan_text_to_string_in(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfilemoddate_in(&mut st.strings, s) }?;
            return ins_file_query_result_in(st, &saved, s, b);
        }
        PDF_FILE_SIZE_CODE => {
            let saved = save_scan_in(st)?;
            let s = scan_text_to_string_in(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfilesize_in(&mut st.strings, s) }?;
            return ins_file_query_result_in(st, &saved, s, b);
        }
        PDF_MDFIVE_SUM_CODE => {
            let saved = save_scan_in(st)?;
            let is_file = crate::scan::scan_keyword_in(st, c"file")?;
            let s = scan_text_to_string_in(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getmd5sum_in(&mut st.strings, s, is_file) };
            return ins_file_query_result_in(st, &saved, s, b);
        }
        PDF_FILE_DUMP_CODE => {
            let saved = save_scan_in(st)?;
            let offset = scan_nonnegative_int_in(
                st,
                c"offset",
                c"Bad file offset",
                "A file offset must be between 0 and 2^_31_-1,",
            )?;
            let length = scan_nonnegative_int_in(
                st,
                c"length",
                c"Bad dump length",
                "A dump length must be between 0 and 2^_31_-1,",
            )?;
            let s = scan_text_to_string_in(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { crate::texmfmp::getfiledump_in(&mut st.strings, s, offset, length) }?;
            return ins_file_query_result_in(st, &saved, s, b);
        }
        PDF_STRCMP_CODE => {
            let saved = save_scan_in(st)?;
            crate::scan::compare_strings_in(st)?;
            restore_scan(st, &saved);
            unseal_pending_str(st, &saved);
        }
        XETEX_UCHAR_CODE => crate::scan::scan_usv_num_in(st)?,
        XETEX_UCHARCAT_CODE => {
            crate::scan::scan_usv_num_in(st)?;
            let saved_chr = st.scanner.cur_val;
            scan_int_in(st)?;
            let code = st.scanner.cur_val;
            if code < LEFT_BRACE as i32
                || code > ACTIVE_CHAR as i32
                || code == OUT_PARAM as i32
                || code == IGNORE as i32
            {
                error_here_with_diagnostic_in(st, c"Invalid code (".as_ptr());
                print_int_in(st, code);
                print_cstr_in(
                    st,
                    c"), should be in the ranges 1..4, 6..8, 10..13".as_ptr(),
                );
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors
                    .help(&["I'm going to use 12 instead of that illegal code value."]);
                scc_err(error_in(st))?;
                cat = 12;
            } else {
                cat = code;
            }
            st.scanner.cur_val = saved_chr;
        }
        XETEX_VARIATION_NAME_CODE => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            if st.fonts.font_area[fnt as usize] == AAT_FONT_FLAG {
                scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
                arg2 = 0;
            } else {
                not_font_error_in(st, c, fnt, c"; not an AAT font")?;
            }
        }
        XETEX_FEATURE_NAME_CODE | XETEX_SELECTOR_NAME_CODE => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            if is_aat_or_graphite(st, fnt) {
                scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
                if c == XETEX_SELECTOR_NAME_CODE {
                    scan_int_in(st)?;
                    arg2 = st.scanner.cur_val;
                } else {
                    arg2 = 0;
                }
            } else {
                not_font_error_in(st, c, fnt, c"; not an AAT or Graphite font")?;
            }
        }
        XETEX_GLYPH_NAME_CODE => {
            crate::scan::scan_font_ident_in(st)?;
            fnt = st.scanner.cur_val;
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                scan_int_in(st)?;
                arg1 = st.scanner.cur_val;
            } else {
                not_font_error_in(st, c, fnt, c"; not a native platform font")?;
            }
        }
        JOB_NAME_CODE => {
            if st.strings.job_name == 0 {
                unsafe { crate::io::open_log_file_in(st) }.map_err(SccAbort::OpenLog)?;
            }
        }
        _ => {}
    }

    let old_selector = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let b = st.strings.pool_ptr;

    match c {
        NUMBER_CODE | PDF_STRCMP_CODE => print_int_in(st, st.scanner.cur_val),
        ROMAN_NUMERAL_CODE => crate::output::print_roman_int_in(st, st.scanner.cur_val),
        STRING_CODE => {
            if st.scanner.cur_cs != 0 {
                sprint_cs_in(st, st.scanner.cur_cs);
            } else {
                print_char_in(st, st.scanner.cur_chr);
            }
        }
        MEANING_CODE => crate::output::print_meaning_in(st),
        FONT_NAME_CODE => {
            let f = st.scanner.cur_val as usize;
            let name = st.fonts.font_name[f];
            let fa = st.fonts.font_area[f];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                let start = st.strings.str_start[(name - TOO_BIG_CHAR) as usize] as usize;
                let len = crate::string_pool::length(&st.strings, name).max(0) as usize;
                let quote_char = if st.strings.str_pool[start..start + len].contains(&(b'"' as u16))
                {
                    '\'' as i32
                } else {
                    '"' as i32
                };
                print_char_in(st, quote_char);
                crate::output::print_in(st, name);
                print_char_in(st, quote_char);
            } else {
                crate::output::print_in(st, name);
            }
            if st.fonts.font_size[f] != st.fonts.font_dsize[f] {
                print_cstr_in(st, c" at ".as_ptr());
                print_scaled_in(st, st.fonts.font_size[f]);
                print_cstr_in(st, c"pt".as_ptr());
            }
        }
        ETEX_REVISION_CODE => print_cstr_in(st, c".6".as_ptr()),
        LEFT_MARGIN_KERN_CODE | RIGHT_MARGIN_KERN_CODE => {
            let list = margin_list;
            if c == LEFT_MARGIN_KERN_CODE {
                print_margin_kern_in(st, list, 0, GLUE_PAR_LEFT_SKIP, |mem, p| mem.b32s1(p));
            } else {
                let last = crate::mem::prev_rightmost(&st.mem, MemPtr(list), MemPtr::NULL).0;
                print_margin_kern_in(st, last, 1, GLUE_PAR_RIGHT_SKIP, move |mem, p| {
                    crate::mem::prev_rightmost(mem, MemPtr(list), MemPtr(p)).0
                });
            }
        }
        UNIFORM_DEVIATE_CODE => {
            let v = crate::scaled_math::unif_rand(&mut st.errors, st.scanner.cur_val);
            print_int_in(st, v);
        }
        NORMAL_DEVIATE_CODE => {
            let v = crate::scaled_math::norm_rand(st);
            print_int_in(st, v);
        }
        XETEX_UCHAR_CODE | XETEX_UCHARCAT_CODE => print_char_in(st, st.scanner.cur_val),
        XETEX_REVISION_CODE => print_cstr_in(st, c".999994".as_ptr()),
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
                unsafe { crate::ext::gr_print_font_name_in(st, c, engine, arg1, arg2) };
            }
        }
        XETEX_GLYPH_NAME_CODE => {
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { crate::ext::print_glyph_name_in(st, fnt, arg1) }?;
            }
        }
        JOB_NAME_CODE => {
            let job_name = st.strings.job_name;
            crate::output::print_file_name_in(st, job_name, 0, 0);
        }
        _ => {}
    }

    st.shipout.selector = old_selector;
    let tail = str_toks_scc(st, b, cat as i16)?;
    ins_temp_head_list(st, tail)
}

fn is_aat_or_graphite(st: &EngineState, f: i32) -> bool {
    let fa = st.fonts.font_area[f as usize];
    fa == AAT_FONT_FLAG
        || (fa == OTGR_FONT_FLAG && unsafe { usingGraphite(st.fonts.font_layout_engine_ptr(f)) })
}

fn not_font_error_in(
    st: &mut EngineState,
    c: i32,
    f: i32,
    msg: &std::ffi::CStr,
) -> Result<(), SccAbort> {
    scc_err(crate::errors::not_font_error_common_in(
        st,
        CONVERT,
        c,
        f,
        msg.as_ptr(),
    ))
}

fn make_string_overflow_scc(st: &EngineState) -> SccAbort {
    SccAbort::Input(InputAbort::Overflow {
        what: c"number of strings".as_ptr(),
        n: st.strings.max_strings - st.strings.init_str_ptr,
    })
}

/// Puts `token` in front of the backed-up token list on top of the input stack.
fn prepend_to_backed_up_in(st: &mut EngineState, token: i32) -> Result<(), SccAbort> {
    let p = get_avail_scc(st)?;
    p.set_info(&mut st.mem, token);
    p.set_link(&mut st.mem, MemPtr::from_raw(st.input.cur_input.loc));
    st.input.cur_input.start = p.0;
    st.input.cur_input.loc = p.0;
    Ok(())
}

pub(crate) fn expand_in(st: &mut EngineState) -> Result<(), SccAbort> {
    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(SccAbort::Input(InputAbort::Overflow {
            what: c"expansion depth".as_ptr(),
            n: st.scanner.expand_depth,
        }));
    }

    let cv_backup = st.scanner.cur_val;
    let cvl_backup = st.scanner.cur_val_level;
    let radix_backup = st.scanner.radix;
    let co_backup = st.scanner.cur_order;
    let backup_backup = st.mem.b32s1(BACKUP_HEAD);

    'reswitch: loop {
        if st.scanner.cur_cmd >= CALL {
            if st.scanner.cur_cmd < END_TEMPLATE {
                macro_call_in(st)?;
            } else {
                st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_ENDV;
                back_input_in(st)?;
            }
            break;
        }
        if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
            show_cur_cmd_chr_in(st);
        }
        match st.scanner.cur_cmd {
            TOP_BOT_MARK => {
                let t = st.scanner.cur_chr % MARKS_CODE;
                if st.scanner.cur_chr >= MARKS_CODE {
                    crate::scan::scan_register_num_in(st)?;
                } else {
                    st.scanner.cur_val = 0;
                }
                if st.scanner.cur_val == 0 {
                    st.scanner.cur_ptr = st.scanner.cur_mark[t as usize];
                } else {
                    let n = st.scanner.cur_val;
                    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, MARK_VAL, n, false);
                    if let Some(sa) = NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr) {
                        st.scanner.cur_ptr = sa.child(&st.mem, t);
                    }
                }
                if !st.scanner.cur_ptr.is_null() {
                    begin_token_list_in(st, st.scanner.cur_ptr.0, MARK_TEXT)?;
                }
            }
            EXPAND_AFTER => {
                if st.scanner.cur_chr == 0 {
                    get_token_in(st)?;
                    let t = st.scanner.cur_tok;
                    get_token_in(st)?;
                    if st.scanner.cur_cmd > MAX_COMMAND {
                        expand_in(st)?;
                    } else {
                        back_input_in(st)?;
                    }
                    st.scanner.cur_tok = t;
                    back_input_in(st)?;
                } else {
                    get_token_in(st)?;
                    if st.scanner.cur_cmd == IF_TEST && st.scanner.cur_chr != IF_CASE_CODE as i32 {
                        st.scanner.cur_chr += UNLESS_CODE;
                        continue 'reswitch;
                    }
                    error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                    print_esc_cstr_in(st, c"unless".as_ptr());
                    print_cstr_in(st, c"' before `".as_ptr());
                    print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                    print_char_in(st, '\'' as i32);
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors
                        .help(&["Continue, and I'll forget that it ever happened."]);
                    scc_err(back_error_in(st)?)?;
                }
            }
            NO_EXPAND => {
                if st.scanner.cur_chr == 0 {
                    get_token_unexpanded_in(st)?;
                    let t = st.scanner.cur_tok;
                    back_input_in(st)?;
                    if t >= CS_TOKEN_FLAG {
                        prepend_to_backed_up_in(st, CS_TOKEN_FLAG + FROZEN_DONT_EXPAND)?;
                    }
                } else {
                    get_token_unexpanded_in(st)?;
                    st.scanner.cur_cs = crate::hash::prim_lookup_cs(
                        &mut st.eqtb,
                        &mut st.strings,
                        st.scanner.cur_cs,
                    )
                    .map_err(|e| SccAbort::Input(crate::scan::hash_err_abort(e)))?;
                    if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                        let cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
                        if cmd > MAX_COMMAND {
                            st.scanner.cur_cmd = cmd;
                            st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
                            st.scanner.cur_tok =
                                st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
                            st.scanner.cur_cs = 0;
                            continue 'reswitch;
                        }
                        back_input_in(st)?;
                        prepend_to_backed_up_in(st, CS_TOKEN_FLAG + FROZEN_PRIMITIVE)?;
                    }
                }
            }
            CS_NAME => {
                let head = scan_cs_name_tokens_in(st)?;
                let first = st.input.first;
                let j = token_chars_to_buffer(st, head.link(&st.mem))?;
                st.scanner.cur_cs = if j > first + 1 || st.input.buffer[first as usize] > 65535 {
                    st.eqtb.no_new_control_sequence = false;
                    let cs = crate::hash::id_lookup(
                        &mut st.eqtb,
                        &mut st.strings,
                        &st.input.buffer,
                        first,
                        j - first,
                    )
                    .map_err(|e| SccAbort::Input(crate::scan::hash_err_abort(e)))?;
                    st.eqtb.no_new_control_sequence = true;
                    cs
                } else if j == first {
                    NULL_CS
                } else {
                    SINGLE_BASE + st.input.buffer[first as usize]
                };
                crate::mem::flush_list(&mut st.mem, head);
                let cs = st.scanner.cur_cs;
                if st.eqtb.eq_type(cs) == UNDEFINED_CS {
                    let _ = crate::save::eq_define(
                        &mut st.eqtb,
                        &mut st.save_stack,
                        &mut st.mem,
                        &mut st.scanner,
                        cs,
                        RELAX,
                        TOO_BIG_USV,
                    );
                }
                st.scanner.cur_tok = st.scanner.cur_cs + CS_TOKEN_FLAG;
                back_input_in(st)?;
            }
            CONVERT => conv_toks_in(st)?,
            THE => ins_the_toks_in(st)?,
            IF_TEST => conditional_in(st)?,
            FI_OR_ELSE => {
                if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0
                    && st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) <= 1
                {
                    show_cur_cmd_chr_in(st);
                }
                if st.scanner.cur_chr > st.scanner.if_limit as i32 {
                    if st.scanner.if_limit == IF_CODE {
                        crate::input::insert_relax_in(st)?;
                    } else {
                        error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
                        print_cmd_chr_in(st, FI_OR_ELSE, st.scanner.cur_chr);
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors
                            .help(&[r"I'm ignoring this; it doesn't match any \if."]);
                        scc_err(error_in(st))?;
                    }
                } else {
                    while st.scanner.cur_chr != FI_CODE as i32 {
                        crate::scan::pass_text_in(st)?;
                    }
                    pop_fi_in(st);
                }
            }
            INPUT => {
                if st.scanner.cur_chr == 1 {
                    st.input.force_eof = true;
                } else if st.scanner.cur_chr == 2 {
                    crate::scan::pseudo_start_in(st)?;
                } else if st.scanner.name_in_progress {
                    crate::input::insert_relax_in(st)?;
                } else {
                    unsafe { crate::io::start_input_in(st, std::ptr::null()) }?;
                }
            }
            _ => {
                print_err_in(st, c"Undefined control sequence");
                st.errors.help(&[
                    "The control sequence at the end of the top line",
                    r"of your error message was never \def'ed. If you have",
                    r"misspelled it (e.g., `\hobx'), type `I' and the correct",
                    r"spelling (e.g., `I\hbox'). Otherwise just continue,",
                    "and I'll forget about whatever was undefined.",
                ]);
                scc_err(error_in(st))?;
            }
        }
        break;
    }

    st.scanner.cur_val = cv_backup;
    st.scanner.cur_val_level = cvl_backup;
    st.scanner.radix = radix_backup;
    st.scanner.cur_order = co_backup;
    st.mem.set_b32s1(BACKUP_HEAD, backup_backup);
    st.scanner.expand_depth_count -= 1;
    Ok(())
}

/// Reports a `\par` inside a non-`\long` macro argument and discards the arguments so far.
#[cold]
fn runaway_par_in_argument_in(
    st: &mut EngineState,
    pstack: &mut [i32; 9],
    n: i32,
    unbalance: i32,
) -> Result<(), SccAbort> {
    if st.scanner.long_state == CALL {
        crate::errors::runaway_in(st);
        error_here_with_diagnostic_in(st, c"Paragraph ended before ".as_ptr());
        sprint_cs_in(st, st.scanner.warning_index);
        print_cstr_in(st, c" was complete".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help(&[
            "I suspect you've forgotten a `}', causing me to apply this",
            "control sequence to too much text. How can we recover?",
            "My plan is to forget the whole thing and hope for the best.",
        ]);
        scc_err(back_error_in(st)?)?;
    }
    pstack[n as usize] = st.mem.b32s1(TEMP_HEAD);
    st.scanner.align_state -= unbalance;
    for &list in &pstack[..=n as usize] {
        crate::mem::flush_list(&mut st.mem, MemPtr::from_raw(list));
    }
    Ok(())
}

#[cold]
fn extra_right_brace_in_argument_in(st: &mut EngineState) -> Result<(), SccAbort> {
    back_input_in(st)?;
    error_here_with_diagnostic_in(st, c"Argument of ".as_ptr());
    sprint_cs_in(st, st.scanner.warning_index);
    print_cstr_in(st, c" has an extra }".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
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
    scc_err(crate::input::ins_error_in(st)?)
}

#[cold]
fn trace_macro_call_in(st: &mut EngineState, ref_count: i32) {
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    let stack_levels = st.eqtb.int_par(INT_PAR_TRACING_STACK_LEVELS);
    if stack_levels > 0 {
        if st.input.input_ptr < stack_levels {
            crate::output::print_ln_in(st);
            print_char_in(st, b'~' as i32);
            for _ in 0..st.input.input_ptr {
                print_char_in(st, b'.' as i32);
            }
            crate::output::print_cs_in(st, st.scanner.warning_index);
            crate::output::token_show_in(st, ref_count);
        } else {
            print_char_in(st, b'~' as i32);
            print_char_in(st, b'~' as i32);
            crate::output::print_cs_in(st, st.scanner.warning_index);
        }
    } else {
        crate::output::print_ln_in(st);
        crate::output::print_cs_in(st, st.scanner.warning_index);
        crate::output::token_show_in(st, ref_count);
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, false);
}

#[cold]
fn trace_macro_param_in(st: &mut EngineState, match_chr: i32, n: i32, list: i32) {
    let stack_levels = st.eqtb.int_par(INT_PAR_TRACING_STACK_LEVELS);
    if stack_levels == 0 || st.input.input_ptr < stack_levels {
        begin_diagnostic_in(st);
        diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_nl_in(st, match_chr);
        print_int_in(st, n);
        print_cstr_in(st, c"<-".as_ptr());
        crate::output::show_token_list_in(st, list, TEX_NULL, 1000);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        end_diagnostic_in(st, false);
    }
}

pub(crate) fn macro_call_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;
    let ref_count = st.scanner.cur_chr;
    let mut r = st.mem.b32s1(ref_count);
    let mut n: i32 = 0;

    if st.eqtb.int_par(INT_PAR_TRACING_MACROS) > 0 {
        trace_macro_call_in(st, ref_count);
    }

    if crate::scan::is_protected_macro(&st.mem, ref_count) {
        r = st.mem.b32s1(r);
    }

    let mut pstack: [i32; 9] = [TEX_NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr: i32 = TEX_NULL;

    'exit: {
        if st.mem.b32s0(r) != END_MATCH_TOKEN {
            st.scanner.scanner_status = MATCHING;
            let mut unbalance: i32 = 0;
            st.scanner.long_state = st.eqtb.eq_type(st.scanner.cur_cs);
            if st.scanner.long_state >= OUTER_CALL {
                st.scanner.long_state -= 2;
            }

            loop {
                st.mem.set_b32s1(TEMP_HEAD, TEX_NULL);
                let s: i32;
                let mut p: i32 = TEX_NULL;
                let mut m: i32 = 0;
                if st.mem.b32s0(r) >= END_MATCH_TOKEN || st.mem.b32s0(r) < MATCH_TOKEN {
                    s = TEX_NULL;
                } else {
                    match_chr = st.mem.b32s0(r) - MATCH_TOKEN;
                    s = st.mem.b32s1(r);
                    r = s;
                    p = TEMP_HEAD.0;
                }

                'continue_: loop {
                    get_token_in(st)?;

                    if st.scanner.cur_tok == st.mem.b32s0(r) {
                        r = st.mem.b32s1(r);
                        if st.mem.b32s0(r) >= MATCH_TOKEN && st.mem.b32s0(r) <= END_MATCH_TOKEN {
                            if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                                st.scanner.align_state -= 1;
                            }
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    if s != r {
                        if s == TEX_NULL {
                            error_here_with_diagnostic_in(st, c"Use of ".as_ptr());
                            sprint_cs_in(st, st.scanner.warning_index);
                            print_cstr_in(st, c" doesn't match its definition".as_ptr());
                            capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help(&[
                                r"If you say, e.g., `\def\a1{...}', then you must always",
                                r"put `1' after `\a', since control sequence names are",
                                "made up of letters only. The macro here has not been",
                                "followed by the required stuff, so I'm ignoring it.",
                            ]);
                            scc_err(error_in(st))?;
                            break 'exit;
                        }
                        let mut t = s;
                        loop {
                            let q = get_avail_scc(st)?.0;
                            st.mem.set_b32s1(p, q);
                            st.mem.set_b32s0(q, st.mem.b32s0(t));
                            p = q;
                            m += 1;
                            let mut u = st.mem.b32s1(t);
                            let mut v = s;
                            loop {
                                if u == r {
                                    if st.scanner.cur_tok != st.mem.b32s0(v) {
                                        break;
                                    }
                                    r = st.mem.b32s1(v);
                                    continue 'continue_;
                                }
                                if st.mem.b32s0(u) != st.mem.b32s0(v) {
                                    break;
                                }
                                u = st.mem.b32s1(u);
                                v = st.mem.b32s1(v);
                            }
                            t = st.mem.b32s1(t);
                            if t == r {
                                break;
                            }
                        }
                        r = s;
                    }

                    if st.scanner.cur_tok == st.input.par_token
                        && st.scanner.long_state != LONG_CALL
                    {
                        runaway_par_in_argument_in(st, &mut pstack, n, unbalance)?;
                        break 'exit;
                    }

                    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                        if st.scanner.cur_tok >= LEFT_BRACE_LIMIT {
                            extra_right_brace_in_argument_in(st)?;
                            continue 'continue_;
                        }
                        unbalance = 1;
                        loop {
                            let q = get_avail_scc(st)?;
                            st.mem.set_b32s1(p, q.0);
                            q.set_info(&mut st.mem, st.scanner.cur_tok);
                            p = q.0;

                            get_token_in(st)?;

                            if st.scanner.cur_tok == st.input.par_token
                                && st.scanner.long_state != LONG_CALL
                            {
                                runaway_par_in_argument_in(st, &mut pstack, n, unbalance)?;
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
                        let q = get_avail_scc(st)?.0;
                        st.mem.set_b32s1(p, q);
                        st.mem.set_b32s0(q, st.scanner.cur_tok);
                        p = q;
                    } else {
                        if st.scanner.cur_tok == SPACE_TOKEN
                            && st.mem.b32s0(r) <= END_MATCH_TOKEN
                            && st.mem.b32s0(r) >= MATCH_TOKEN
                        {
                            continue 'continue_;
                        }
                        let q = get_avail_scc(st)?.0;
                        st.mem.set_b32s1(p, q);
                        st.mem.set_b32s0(q, st.scanner.cur_tok);
                        p = q;
                    }

                    m += 1;
                    if st.mem.b32s0(r) > END_MATCH_TOKEN || st.mem.b32s0(r) < MATCH_TOKEN {
                        continue 'continue_;
                    }
                    break 'continue_;
                }

                if s != TEX_NULL {
                    if m == 1 && st.mem.b32s0(p) < RIGHT_BRACE_LIMIT {
                        // Strip the braces around a single braced argument.
                        st.mem.set_b32s1(rbrace_ptr, TEX_NULL);
                        crate::mem::free_avail(&mut st.mem, MemPtr::from_raw(p));
                        p = st.mem.b32s1(TEMP_HEAD);
                        pstack[n as usize] = st.mem.b32s1(p);
                        crate::mem::free_avail(&mut st.mem, MemPtr::from_raw(p));
                    } else {
                        pstack[n as usize] = st.mem.b32s1(TEMP_HEAD);
                    }
                    n += 1;
                    if st.eqtb.int_par(INT_PAR_TRACING_MACROS) > 0 {
                        trace_macro_param_in(st, match_chr, n, pstack[(n - 1) as usize]);
                    }
                }

                if st.mem.b32s0(r) == END_MATCH_TOKEN {
                    break;
                }
            }
        }

        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.loc == TEX_NULL
            && st.input.cur_input.index != V_TEMPLATE
        {
            crate::input::end_token_list_in(st)?;
        }

        begin_token_list_in(st, ref_count, MACRO)?;
        st.input.cur_input.name = st.scanner.warning_index;
        st.input.cur_input.loc = st.mem.b32s1(r);

        if n > 0 {
            if st.input.param_ptr + n > st.input.max_param_stack {
                st.input.max_param_stack = st.input.param_ptr + n;
                if st.input.max_param_stack > st.input.param_size {
                    return Err(SccAbort::Input(InputAbort::Overflow {
                        what: c"parameter stack size".as_ptr(),
                        n: st.input.param_size,
                    }));
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
