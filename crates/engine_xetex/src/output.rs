//! Basic printing: numbers, strings, control sequences and the diagnostic sink.

use crate::bridge::{diag_append, diag_append_byte, diag_finish, output_putc, Diagnostic};
use crate::cmd::Cmd;
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BIGGEST_USV, EQTB_SIZE, HASH_BASE, HISTORY_SPOTLESS,
    HISTORY_WARNING_ISSUED, NULL_CS, SCRIPT_SIZE, SINGLE_BASE, TEXT_SIZE, TOO_BIG_CHAR,
    UNDEFINED_CONTROL_SEQUENCE,
};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::node::Stored;
use crate::node::{
    CharNode, DelimiterField, GlueSpecNode, MathField, NodePtr, RuleDimen, SaNode, WhatsitNode,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::save::DIMEN_VAL_LIMIT;
use crate::state::{EngineState, GroupCode, Selector};
use crate::token::TokenList;
use crate::units::{EqtbPtr, FontId, GlueOrder, MemPtr, PrimNum, Scaled, StrNumber};

mod cmd_chr;
mod show;
mod show_box;
mod warnings;

pub(crate) use self::cmd_chr::{print_cmd_chr, print_meaning};
pub(crate) use self::show::{
    issue_message, show_context, show_cur_cmd_chr, show_save_groups, show_token_list, show_whatever,
};
pub(crate) use self::show_box::{short_display, show_box};
pub(crate) use self::warnings::{
    bad_utf8_warning, char_warning, file_warning, font_feature_warning, font_mapping_warning,
    group_warning, if_warning,
};

fn print_the_digs(st: &mut EngineState, mut k: u8) {
    while k > 0 {
        k -= 1;
        let d = i32::from(st.errors.dig[k as usize]);
        if d < 10 {
            print_char(st, i32::from(b'0') + d);
        } else {
            print_char(st, i32::from(b'A') - 10 + d);
        }
    }
}

/// Stores the base-`radix` digits of nonnegative `n` into `dig[k..]`, returning the new length.
fn store_digits(st: &mut EngineState, mut n: i32, radix: i32, mut k: u8) -> u8 {
    loop {
        st.errors.dig[k as usize] = (n % radix) as u8;
        n /= radix;
        k += 1;
        if n == 0 {
            return k;
        }
    }
}

pub(crate) fn print_int(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;
    if n < 0 {
        print_char(st, b'-');
        if n > -100_000_000 {
            n = -n;
        } else {
            // Peel off the last digit so negating cannot overflow.
            let m_val = -1 - n;
            n = m_val / 10;
            let m = (m_val % 10) + 1;
            k = 1;
            if m < 10 {
                st.errors.dig[0] = m as u8;
            } else {
                st.errors.dig[0] = 0;
                n += 1;
            }
        }
    }
    let k = store_digits(st, n, 10, k);
    print_the_digs(st, k);
}

pub(crate) fn print_hex(st: &mut EngineState, n: i32) {
    print_char(st, b'"');
    let k = store_digits(st, n, 16, 0);
    print_the_digs(st, k);
}

pub(crate) fn print_ucs_code(st: &mut EngineState, n: i32) {
    print(st, "U+");
    let mut k = store_digits(st, n, 16, 0);
    while k < 4 {
        st.errors.dig[k as usize] = 0;
        k += 1;
    }
    print_the_digs(st, k);
}

/// A printable TeX string: a pool string number, or literal bytes printed one
/// `print_char` each.
pub(crate) trait TexStr: Copy {
    fn print_to(self, st: &mut EngineState);
}

impl TexStr for i32 {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, self);
    }
}

impl TexStr for StrNumber {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, self.raw());
    }
}

/// A byte prints as the single-character string of that code.
impl TexStr for u8 {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_pool_str(st, i32::from(self));
    }
}

impl TexStr for &str {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_bytes(st, self.as_bytes());
    }
}

impl TexStr for &[u8] {
    #[inline]
    fn print_to(self, st: &mut EngineState) {
        print_bytes(st, self);
    }
}

fn print_bytes(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_char(st, i32::from(b));
    }
}

fn at_line_start(st: &EngineState) -> bool {
    let sel = st.shipout.selector;
    !((st.shipout.term_offset > 0 && sel.raw() % 2 == 1)
        || (st.shipout.file_offset > 0 && sel >= Selector::LogOnly))
}

pub(crate) fn print_nl(st: &mut EngineState, s: impl TexStr) {
    if !at_line_start(st) {
        print_ln(st);
    }
    print(st, s);
}

fn print_escape_char(st: &mut EngineState) {
    let c = st.eqtb.int_par(IntPar::EscapeChar);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char(st, c);
    }
}

pub(crate) fn print_esc(st: &mut EngineState, s: impl TexStr) {
    print_escape_char(st);
    print(st, s);
}

pub(crate) fn print_roman_int(st: &mut EngineState, mut n: i32) {
    // Letter/divisor pairs: each letter is the previous value divided by the digit after it.
    const ROMAN_DATA: &[u8] = b"m2d5c2l5x2v5i";
    let mut j: usize = 0;
    let mut v: i32 = 1000;
    loop {
        while n >= v {
            print_char(st, i32::from(ROMAN_DATA[j]));
            n -= v;
        }
        if n <= 0 {
            return;
        }
        let mut k = j + 2;
        let mut u = v / (i32::from(ROMAN_DATA[k - 1]) - i32::from(b'0'));
        if ROMAN_DATA[k - 1] == b'2' {
            k += 2;
            u /= i32::from(ROMAN_DATA[k - 1]) - i32::from(b'0');
        }
        if n + u >= v {
            print_char(st, i32::from(ROMAN_DATA[k]));
            n += u;
        } else {
            j += 2;
            v /= i32::from(ROMAN_DATA[j - 1]) - i32::from(b'0');
        }
    }
}

pub(crate) fn print_scaled(st: &mut EngineState, s: Scaled) {
    let mut s = s.raw();
    if s < 0 {
        print_char(st, b'-');
        s = s.wrapping_neg();
    }
    print_int(st, s / 0x10000);
    print_char(st, b'.');
    s = 10 * (s % 0x10000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x10000 {
            s = s + 0x8000 - 50000;
        }
        print_char(st, i32::from(b'0') + s / 0x10000);
        s = 10 * (s % 0x10000);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
}

fn warn_char(st: &mut EngineState, c: i32) {
    if let Some(diag) = &mut st.shipout.current_diagnostic {
        diag_append_byte(diag, c as u8);
    }
}

/// Write one transcript byte, counting it so a resumed run can continue the transcript.
pub(crate) fn log_putc(st: &mut EngineState, byte: u8) {
    output_putc(st.shipout.log_file, byte);
    st.shipout.log_bytes += 1;
}

pub(crate) fn print_ln(st: &mut EngineState) {
    match st.shipout.selector {
        sel @ (Selector::TermOnly | Selector::LogOnly | Selector::TermAndLog) => {
            warn_char(st, i32::from(b'\n'));
            if sel != Selector::LogOnly {
                output_putc(st.shipout.rust_stdout, b'\n');
            }
            if sel != Selector::TermOnly {
                log_putc(st, b'\n');
            }
            if sel != Selector::LogOnly {
                st.shipout.term_offset = 0;
            }
            if sel != Selector::TermOnly {
                st.shipout.file_offset = 0;
            }
        }
        Selector::NoPrint | Selector::Pseudo | Selector::NewString => {}
        Selector::File(j) => {
            output_putc(st.shipout.write_file[usize::from(j)], b'\n');
            st.shipout.write_prefix[usize::from(j)].push(i32::from(b'\n'));
        }
    }
}

/// Writes one byte, UTF-16 code unit or (when pseudoprinting) character to the current selector.
pub(crate) fn print_raw_char(st: &mut EngineState, s: impl Into<i32>, incr_offset: bool) {
    let s = s.into();
    match st.shipout.selector {
        sel @ (Selector::TermOnly | Selector::LogOnly | Selector::TermAndLog) => {
            let to_term = sel != Selector::LogOnly;
            let to_log = sel != Selector::TermOnly;
            warn_char(st, s);
            if to_term {
                output_putc(st.shipout.rust_stdout, s as u8);
            }
            if to_log {
                log_putc(st, s as u8);
            }
            if incr_offset {
                if to_term {
                    st.shipout.term_offset += 1;
                }
                if to_log {
                    st.shipout.file_offset += 1;
                }
            }
            if to_term && st.shipout.term_offset == st.config.max_print_line {
                output_putc(st.shipout.rust_stdout, b'\n');
                st.shipout.term_offset = 0;
            }
            if to_log && st.shipout.file_offset == st.config.max_print_line {
                log_putc(st, b'\n');
                st.shipout.file_offset = 0;
            }
        }
        Selector::NoPrint => {}
        Selector::Pseudo => {
            let t = st.errors.tally;
            if t < st.errors.trick_count {
                st.errors.trick_buf[(t % st.errors.error_line) as usize] = s;
            }
        }
        Selector::NewString => {
            let pp = st.strings.pool_ptr;
            if pp < st.strings.pool_size {
                st.strings.str_pool[pp.index()] = s as u16;
                st.strings.pool_ptr = pp + 1;
            }
        }
        Selector::File(j) => {
            output_putc(st.shipout.write_file[usize::from(j)], s as u8);
            st.shipout.write_prefix[usize::from(j)].push(s);
        }
    }
    st.errors.tally += 1;
}

pub(crate) fn print_char(st: &mut EngineState, s: impl Into<i32>) {
    let s = s.into();
    if st.shipout.selector == Selector::NewString && !st.shipout.doing_special {
        if s >= 0x10000 {
            print_raw_char(st, 0xD800 + (s - 0x10000) / 1024, true);
            print_raw_char(st, 0xDC00 + (s - 0x10000) % 1024, true);
        } else {
            print_raw_char(st, s, true);
        }
        return;
    }

    if s == st.eqtb.int_par(IntPar::NewLineChar) && st.shipout.selector < Selector::Pseudo {
        print_ln(st);
        return;
    }

    if s < 32 && !st.shipout.doing_special {
        print_raw_char(st, b'^', true);
        print_raw_char(st, b'^', true);
        print_raw_char(st, s + 64, true);
    } else if s < 127 {
        print_raw_char(st, s, true);
    } else if s == 127 {
        if !st.shipout.doing_special {
            print_raw_char(st, b'^', true);
            print_raw_char(st, b'^', true);
            print_raw_char(st, b'?', true);
        } else {
            print_raw_char(st, s, true);
        }
    } else if s < 160 && !st.shipout.doing_special {
        let hex_digit = |d: i32| {
            if d < 10 {
                i32::from(b'0') + d
            } else {
                i32::from(b'a') + d - 10
            }
        };
        print_raw_char(st, b'^', true);
        print_raw_char(st, b'^', true);
        print_raw_char(st, hex_digit((s % 256) / 16), true);
        print_raw_char(st, hex_digit(s % 16), true);
    } else if st.shipout.selector == Selector::Pseudo {
        print_raw_char(st, s, true);
    } else {
        // UTF-8, counting one column per character.
        if s < 2048 {
            print_raw_char(st, 192 + s / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        } else if s < 0x10000 {
            print_raw_char(st, 224 + s / 4096, false);
            print_raw_char(st, 128 + (s % 4096) / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        } else {
            print_raw_char(st, 240 + s / 0x40000, false);
            print_raw_char(st, 128 + (s % 0x40000) / 4096, false);
            print_raw_char(st, 128 + (s % 4096) / 64, false);
            print_raw_char(st, 128 + s % 64, true);
        }
    }
}

pub(crate) fn print(st: &mut EngineState, s: impl TexStr) {
    s.print_to(st);
}

fn print_pool_str(st: &mut EngineState, s: i32) {
    if s >= st.strings.str_ptr.raw() {
        return print(st, "???");
    }
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return print(st, "???");
        }
        if st.shipout.selector == Selector::NewString {
            print_char(st, s);
            return;
        }
        if s == st.eqtb.int_par(IntPar::NewLineChar) && st.shipout.selector < Selector::Pseudo {
            print_ln(st);
            return;
        }
        let nl = st.eqtb.int_par(IntPar::NewLineChar);
        st.eqtb.set_int_par(IntPar::NewLineChar, -1);
        print_char(st, s);
        st.eqtb.set_int_par(IntPar::NewLineChar, nl);
        return;
    }

    let std::ops::Range { start: mut i, end } = st.strings.str_range(StrNumber::from_raw(s));
    while i < end {
        let c = st.strings.str_pool[i];
        if i + 1 < end {
            if let Some(pair) = surrogate_pair(c, st.strings.str_pool[i + 1]) {
                print_char(st, pair);
                i += 2;
                continue;
            }
        }
        print_char(st, i32::from(c));
        i += 1;
    }
}

fn is_high_surrogate(c: u16) -> bool {
    (0xD800..0xDC00).contains(&c)
}

fn surrogate_pair(hi: u16, lo: u16) -> Option<i32> {
    (is_high_surrogate(hi) && (0xDC00..0xE000).contains(&lo))
        .then(|| 0x10000 + (i32::from(hi) - 0xD800) * 1024 + (i32::from(lo) - 0xDC00))
}

fn print_current_string(st: &mut EngineState) {
    let start = st.strings.cur_string_start();
    for j in start.until(st.strings.pool_ptr) {
        let c = st.strings.str_pool[j.index()];
        print_char(st, i32::from(c));
    }
}

/// Reports the diagnostic being captured and captures into `diagnostic` from now on.
pub(crate) fn capture_to_diagnostic(st: &mut EngineState, diagnostic: Option<Diagnostic>) {
    if let Some(cur) = std::mem::replace(&mut st.shipout.current_diagnostic, diagnostic) {
        diag_finish(cur);
    }
}

/// Appends `text` to the diagnostic being captured.
pub(crate) fn append_to_diagnostic(st: &mut EngineState, text: &str) {
    if let Some(diag) = &mut st.shipout.current_diagnostic {
        diag_append(diag, text);
    }
}

/// The innermost input level that is a real file, with its current line.
fn current_file_line(st: &EngineState) -> Option<(StrNumber, i32)> {
    let mut level = st.input.in_open;
    while level > 0 && st.input.open_levels[level].full_source_name == StrNumber::NONE {
        level -= 1;
    }
    if level == 0 {
        return None;
    }
    let line = if level == st.input.in_open {
        st.mem.line
    } else {
        st.input.open_levels[level + 1].outer_line
    };
    Some((st.input.open_levels[level].full_source_name, line))
}

fn diagnostic_print_file_line(st: &EngineState, diagnostic: &mut Diagnostic) {
    let Some((name, line)) = current_file_line(st) else {
        diag_append(diagnostic, "!");
        return;
    };
    let name = st.strings.texstring_bytes(name);
    let name = String::from_utf8_lossy(&name);
    diag_append(diagnostic, &format!("{name}:{line}: "));
}

pub(crate) fn diagnostic_begin_capture_warning_here(st: &mut EngineState) {
    let mut warning = Diagnostic::warning();
    diagnostic_print_file_line(st, &mut warning);
    capture_to_diagnostic(st, Some(warning));
}

/// Starts an error with `message`, capturing what follows into its diagnostic.
pub(crate) fn error_here_with_diagnostic(st: &mut EngineState, message: &str) {
    let mut error = Diagnostic::error();
    diagnostic_print_file_line(st, &mut error);
    diag_append(&mut error, message);
    print_error_prefix(st);
    print(st, message);
    capture_to_diagnostic(st, Some(error));
}

/// TeX's `print_err`: starts an error with `message` and closes its diagnostic.
pub(crate) fn print_err(st: &mut EngineState, message: &str) {
    error_here_with_diagnostic(st, message);
    capture_to_diagnostic(st, None);
}

/// Starts an error line with `! ` or, in file:line:error style, the source position.
pub(crate) fn print_error_prefix(st: &mut EngineState) {
    if st.config.file_line_error_style_p != 0 {
        print_file_line(st);
    } else {
        print_nl(st, "! ");
    }
}

pub(crate) fn print_file_line(st: &mut EngineState) {
    let Some((name, line)) = current_file_line(st) else {
        print_nl(st, "! ");
        return;
    };
    print_nl(st, "");
    print(st, name);
    print_char(st, b':');
    print_int(st, line);
    print(st, ": ");
}

pub(crate) fn print_size(st: &mut EngineState, s: i32) {
    print_esc(
        st,
        match s {
            TEXT_SIZE => "textfont",
            SCRIPT_SIZE => "scriptfont",
            _ => "scriptscriptfont",
        },
    );
}

fn print_write_whatsit(st: &mut EngineState, s: &str, p: NodePtr<WhatsitNode>) {
    print_esc(st, s);
    let v = p.write_stream(&st.mem);
    if v < 16 {
        print_int(st, v);
    } else if v == 16 {
        print_char(st, b'*');
    } else {
        print_char(st, b'-');
    }
}

pub(crate) fn print_file_name(st: &mut EngineState, n: StrNumber, a: StrNumber, e: StrNumber) {
    // Swaps one quote character for the other.
    const QUOTE_SUM: i32 = b'"' as i32 + b'\'' as i32;

    let mut must_quote = false;
    let mut quote_char: i32 = 0;
    for s in [a, n, e] {
        if s == StrNumber::NONE {
            continue;
        }
        for j in st.strings.str_range(s) {
            if must_quote && quote_char != 0 {
                break;
            }
            let c = i32::from(st.strings.str_pool[j]);
            if c == i32::from(b' ') {
                must_quote = true;
            } else if c == i32::from(b'"') || c == i32::from(b'\'') {
                must_quote = true;
                quote_char = QUOTE_SUM - c;
            }
        }
    }

    if must_quote {
        if quote_char == 0 {
            quote_char = i32::from(b'"');
        }
        print_char(st, quote_char);
    }

    for s in [a, n, e] {
        if s == StrNumber::NONE {
            continue;
        }
        for j in st.strings.str_range(s) {
            let c = i32::from(st.strings.str_pool[j]);
            if c == quote_char {
                print(st, quote_char);
                quote_char = QUOTE_SUM - quote_char;
                print(st, quote_char);
            }
            print(st, c);
        }
    }

    if quote_char != 0 {
        print_char(st, quote_char);
    }
}

/// The name string of a multi-letter control sequence at eqtb location `p`.
fn cs_text(st: &EngineState, p: EqtbPtr) -> StrNumber {
    match PrimNum::at_eqtb_loc(p) {
        Some(prim) => st.eqtb.prim_text(prim).unwrap_or(StrNumber::from_raw(-1)),
        None => st.eqtb.text(p),
    }
}

pub(crate) fn print_cs(st: &mut EngineState, p: EqtbPtr) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                print_esc(st, "csname");
                print_esc(st, "endcsname");
                print_char(st, b' ');
            } else {
                print_esc(st, p - SINGLE_BASE);
                if st.eqtb.cat_code(p - SINGLE_BASE) == Cmd::Letter {
                    print_char(st, b' ');
                }
            }
        } else if p < ACTIVE_BASE {
            print_esc(st, "IMPOSSIBLE.");
        } else {
            print_char(st, p - ACTIVE_BASE);
        }
    } else if (UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > st.eqtb.eqtb_top {
        print_esc(st, "IMPOSSIBLE.");
    } else if st.eqtb.text(p) >= st.strings.str_ptr {
        print_esc(st, "NONEXISTENT.");
    } else {
        print_esc(st, cs_text(st, p));
        print_char(st, b' ');
    }
}

/// Like [`print_cs`] but without the trailing space.
pub(crate) fn sprint_cs(st: &mut EngineState, p: EqtbPtr) {
    if p >= HASH_BASE {
        print_esc(st, cs_text(st, p));
    } else if p < SINGLE_BASE {
        print_char(st, p - ACTIVE_BASE);
    } else if p < NULL_CS {
        print_esc(st, p - SINGLE_BASE);
    } else {
        print_esc(st, "csname");
        print_esc(st, "endcsname");
    }
}

fn print_native_word(st: &mut EngineState, p: NodePtr<WhatsitNode>) {
    let len = p.native_length(&st.mem) as usize;
    let mut i = 0;
    while i < len {
        let c = p.native_text(&st.mem)[i];
        if i + 1 < len {
            if let Some(pair) = surrogate_pair(c, p.native_text(&st.mem)[i + 1]) {
                print_char(st, pair);
                i += 2;
                continue;
            }
        }
        if is_high_surrogate(c) {
            print(st, b'.');
        } else {
            print_char(st, i32::from(c));
        }
        i += 1;
    }
}

fn print_sa_num(st: &mut EngineState, q: NodePtr<SaNode>) {
    let mem = &st.mem;
    let n = if q.sa_index(mem) < DIMEN_VAL_LIMIT {
        q.sa_num(mem)
    } else {
        let q1 = q.parent(mem);
        let q2 = q1.parent(mem);
        let q3 = q2.parent(mem);
        let digit = |q: NodePtr<SaNode>| i32::from(q.sa_index(mem));
        digit(q) % 64 + 64 * digit(q1) + 64 * 64 * (digit(q2) + 64 * digit(q3))
    };
    print_int(st, n);
}

pub(crate) fn begin_diagnostic(st: &mut EngineState) {
    st.shipout.old_setting = st.shipout.selector;
    if st.eqtb.int_par(IntPar::TracingOnline) <= 0 && st.shipout.selector == Selector::TermAndLog {
        st.shipout.selector = Selector::LogOnly;
        note_warning_issued(st);
    }
}

pub(crate) fn end_diagnostic(st: &mut EngineState, blank_line: bool) {
    print_nl(st, "");
    if blank_line {
        print_ln(st);
    }
    st.shipout.selector = st.shipout.old_setting;
}

fn note_warning_issued(st: &mut EngineState) {
    if st.errors.history == HISTORY_SPOTLESS {
        st.errors.history = HISTORY_WARNING_ISSUED;
    }
}

fn mode_name(m: i32) -> Option<&'static str> {
    let group = i32::from(Cmd::MAX_COMMAND) + 1;
    match m.signum() {
        0 => Some("no mode"),
        1 => ["vertical mode", "horizontal mode", "display math mode"]
            .get((m / group) as usize)
            .copied(),
        _ => [
            "internal vertical mode",
            "restricted horizontal mode",
            "math mode",
        ]
        .get((-m / group) as usize)
        .copied(),
    }
}

pub(crate) fn print_mode(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print(st, name);
    }
}

pub(crate) fn print_in_mode(st: &mut EngineState, m: i32) {
    if let Some(name) = mode_name(m) {
        print(st, "' in ");
        print(st, name);
    }
}

pub(crate) fn print_group(st: &mut EngineState, e: bool) {
    let name = match st.save_stack.cur_group {
        GroupCode::BottomLevel => {
            print(st, "bottom level");
            return;
        }
        GroupCode::Simple => "simple",
        GroupCode::SemiSimple => "semi simple",
        GroupCode::Hbox => "hbox",
        GroupCode::AdjustedHbox => "adjusted hbox",
        GroupCode::Vbox => "vbox",
        GroupCode::Vtop => "vtop",
        GroupCode::Align => "align",
        GroupCode::NoAlign => "no align",
        GroupCode::Output => "output",
        GroupCode::Disc => "disc",
        GroupCode::Insert => "insert",
        GroupCode::Vcenter => "vcenter",
        GroupCode::Math => "math",
        GroupCode::MathChoice => "math choice",
        GroupCode::MathShift => "math shift",
        GroupCode::MathLeft => "math left",
    };
    print(st, name);
    print(st, " group (level ");
    print_int(st, i32::from(st.save_stack.cur_level));
    print_char(st, b')');
    let line_at = st.save_stack.saved(-1);
    if line_at != 0 {
        if e {
            print(st, " entered at line ");
        } else {
            print(st, " at line ");
        }
        print_int(st, line_at);
    }
}

pub(crate) fn token_show(st: &mut EngineState, p: TokenList) {
    if !p.is_null() {
        let head = p.link(&st.mem);
        show_token_list(st, head, TokenList::NULL, 10_000_000);
    }
}

pub(crate) fn print_spec(st: &mut EngineState, p: impl crate::units::MemAddr, s: Option<&str>) {
    let p = MemPtr::from_raw(p.addr());
    let spec = (MemPtr::MEM_BOT..st.mem.lo_mem_max)
        .contains(&p)
        .then(|| NodePtr::<GlueSpecNode>::from_glue_ptr(p));
    let Some(gs) = spec else {
        print_char(st, b'*');
        return;
    };
    print_scaled(st, gs.width(&st.mem));
    if let Some(s) = s {
        print(st, s);
    }
    if gs.stretch(&st.mem) != Scaled::ZERO {
        print(st, " plus ");
        print_glue(st, gs.stretch(&st.mem), gs.stretch_order(&st.mem), s);
    }
    if gs.shrink(&st.mem) != Scaled::ZERO {
        print(st, " minus ");
        print_glue(st, gs.shrink(&st.mem), gs.shrink_order(&st.mem), s);
    }
}

pub(crate) fn print_skip_param(st: &mut EngineState, n: i32) {
    match GluePar::from_raw(n) {
        Some(g) => print_esc(st, g.name()),
        None => print(st, "[unknown glue parameter!]"),
    }
}

pub(crate) fn print_param(st: &mut EngineState, n: i32) {
    match IntPar::from_raw(n).and_then(IntPar::name) {
        Some(name) => print_esc(st, name),
        None => print(st, "[unknown integer parameter!]"),
    }
}

pub(crate) fn print_length_param(st: &mut EngineState, n: i32) {
    match DimenPar::from_raw(n) {
        Some(d) => print_esc(st, d.name()),
        None => print(st, "[unknown dimen parameter!]"),
    }
}

pub(crate) fn print_style(st: &mut EngineState, c: i32) {
    let name = match c / 2 {
        0 => "displaystyle",
        1 => "textstyle",
        2 => "scriptstyle",
        3 => "scriptscriptstyle",
        _ => return print(st, "Unknown style!"),
    };
    print_esc(st, name);
}

/// Prints a character code, as a string number when it is one.
fn print_code(st: &mut EngineState, c: i32) {
    if c < TOO_BIG_CHAR {
        print(st, c);
    } else {
        print_char(st, c);
    }
}

fn print_fam_and_char(st: &mut EngineState, p: NodePtr<MathField>) {
    print_esc(st, "fam");
    print_int(st, p.fam(&st.mem));
    print_char(st, b' ');
    print_code(st, p.full_char(&st.mem));
}

fn print_delimiter(st: &mut EngineState, p: NodePtr<DelimiterField>) {
    let (small_fam, small_char) = p.small(&st.mem);
    let (large_fam, large_char) = p.large(&st.mem);
    let a = ((i64::from(small_fam) * 256 + i64::from(small_char)) * 4096
        + i64::from(large_fam) * 256
        + i64::from(large_char)) as i32;
    if a < 0 {
        print_int(st, a);
    } else {
        print_hex(st, a);
    }
}

pub(crate) fn print_rule_dimen(st: &mut EngineState, d: RuleDimen) {
    match d {
        RuleDimen::Running => print_char(st, b'*'),
        RuleDimen::Set(d) => print_scaled(st, d),
    }
}

/// Prints `d` with its fil/fill/filll order, or the unit `s` for finite glue.
pub(crate) fn print_glue(
    st: &mut EngineState,
    d: Scaled,
    order: impl Into<Stored<GlueOrder>>,
    s: Option<&str>,
) {
    let order = order.into();
    print_scaled(st, d);
    if order > GlueOrder::Filll {
        print(st, "foul");
    } else if order > GlueOrder::Normal {
        print(st, "fil");
        for _ in 1..order.raw() {
            print_char(st, b'l');
        }
    } else if let Some(s) = s {
        print(st, s);
    }
}

fn print_font_char(st: &mut EngineState, font: FontId, c: u16) {
    if font > st.fonts.font_max {
        print_char(st, b'*');
    } else {
        print_esc(st, st.eqtb.font_id_text(font));
    }
    print_char(st, b' ');
    print(st, i32::from(c));
}

fn print_font_and_char(st: &mut EngineState, p: MemPtr) {
    if p > st.mem.mem_end {
        print_esc(st, "CLOBBERED.");
    } else {
        let ch = NodePtr::<CharNode>::at(p);
        print_font_char(st, ch.font(&st.mem), ch.character(&st.mem));
    }
}

fn print_mark(st: &mut EngineState, p: TokenList) {
    print_char(st, b'{');
    if p.ptr() < st.mem.hi_mem_min || p.ptr() > st.mem.mem_end {
        print_esc(st, "CLOBBERED.");
    } else {
        let head = p.link(&st.mem);
        let l = st.config.max_print_line - 10;
        show_token_list(st, head, TokenList::NULL, l);
    }
    print_char(st, b'}');
}

/// `select font <name>`, quoting native font names, plus ` at <size>pt` if scaled.
fn print_font_select(st: &mut EngineState, f: FontId) {
    print(st, "select font ");
    print_font_name_and_size(st, f);
}

/// The name of font `f`, quoted if native, then ` at <size>pt` unless at design size.
pub(crate) fn print_font_name_and_size(st: &mut EngineState, f: FontId) {
    let name = st.fonts.font_name[f];
    let area = st.fonts.font_area[f];
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let has_double_quote =
            st.strings.str_pool[st.strings.str_range(name)].contains(&(u16::from(b'"')));
        let quote_char = i32::from(if has_double_quote { b'\'' } else { b'"' });
        print_char(st, quote_char);
        print(st, name);
        print_char(st, quote_char);
    } else {
        print(st, name);
    }
    if st.fonts.font_size[f] != st.fonts.font_dsize[f] {
        print(st, " at ");
        print_scaled(st, st.fonts.font_size[f]);
        print(st, "pt");
    }
}

fn print_name_of_file(st: &mut EngineState) {
    let name = st.input.name_of_file_bytes().to_vec();
    print_raw_bytes(st, &name);
}

/// Prints `bytes` with `print_raw_char`, one column each.
pub(crate) fn print_raw_bytes(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_raw_char(st, i32::from(b), true);
    }
}
