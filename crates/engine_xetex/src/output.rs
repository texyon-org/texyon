//! Basic printing: numbers, strings, control sequences and the diagnostic sink.

use crate::bridge::output_putc;
use crate::cmd::Cmd;
use crate::consts::{
    BIGGEST_CHAR, BIGGEST_USV, HISTORY_SPOTLESS, HISTORY_WARNING_ISSUED, SCRIPT_SIZE, TEXT_SIZE,
    TOO_BIG_CHAR,
};
use crate::cs::{CsId, CsKind};
use crate::diagnostic::{
    begin_capture, begin_warning, current_file_line, end_capture, DiagnosticKind,
};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::glue_store::GlueSpecId;
use crate::node::Stored;
use crate::node::{CharNode, NativeWordNode, NodePtr, RuleDimen, SaElement};
use crate::node::{DelimiterWord, MathWord};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, GroupCode, Selector};
use crate::token::{TokCursor, TokenList};
use crate::units::{FontId, GlueOrder, MemPtr, Scaled, StrNumber};

mod cmd_chr;
mod show;
mod show_box;
mod warnings;

pub(crate) use self::cmd_chr::{print_command, print_meaning};
pub(crate) use self::show::{
    issue_message, show_context, show_cur_cmd_chr, show_save_groups, show_token_list, show_tokens,
    show_whatever,
};
pub(crate) use self::show_box::{short_display, show_box};
pub(crate) use self::warnings::{
    bad_utf8_warning, char_warning, file_warning, font_feature_warning, font_mapping_warning,
    group_warning, if_warning,
};

/// Prints `n` in base `radix` with at least `min_digits` digits.
fn print_digits(st: &mut EngineState, mut n: u32, radix: u32, min_digits: usize) {
    let mut dig = [0u8; 10];
    let mut k = 0;
    loop {
        dig[k] = (n % radix) as u8;
        n /= radix;
        k += 1;
        if n == 0 {
            break;
        }
    }
    for &d in dig[..k.max(min_digits)].iter().rev() {
        print_char(st, if d < 10 { b'0' + d } else { b'A' - 10 + d });
    }
}

pub(crate) fn print_int(st: &mut EngineState, n: i32) {
    if n < 0 {
        print_char(st, b'-');
    }
    print_digits(st, n.unsigned_abs(), 10, 0);
}

pub(crate) fn print_hex(st: &mut EngineState, n: i32) {
    print_char(st, b'"');
    print_digits(st, n.unsigned_abs(), 16, 0);
}

pub(crate) fn print_ucs_code(st: &mut EngineState, n: i32) {
    print(st, "U+");
    print_digits(st, n.unsigned_abs(), 16, 4);
}

/// A value TeX prints, following `\escapechar`, `\newlinechar` and the selector.
pub(crate) trait TexPrint {
    fn print_to(&self, st: &mut EngineState);
}

impl<T: TexPrint + ?Sized> TexPrint for &T {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        (**self).print_to(st);
    }
}

impl TexPrint for StrNumber {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_pool_str(st, self.raw());
    }
}

/// A byte prints as the single-character string of that code.
impl TexPrint for u8 {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_pool_str(st, i32::from(*self));
    }
}

/// Literal text, one `print_char` per byte.
impl TexPrint for str {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_bytes(st, self.as_bytes());
    }
}

impl TexPrint for [u8] {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_bytes(st, self);
    }
}

/// One character, as `print_char` prints it.
impl TexPrint for char {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_char(st, u32::from(*self).cast_signed());
    }
}

/// A decimal number, as `print_int` prints it.
impl TexPrint for i32 {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_int(st, *self);
    }
}

/// A dimension without its unit, as `print_scaled` prints it.
impl TexPrint for Scaled {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_scaled(st, *self);
    }
}

/// Nothing for `None`.
impl<T: TexPrint> TexPrint for Option<T> {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        if let Some(value) = self {
            value.print_to(st);
        }
    }
}

/// A control sequence as written, as `sprint_cs` prints it.
#[derive(Copy, Clone)]
pub(crate) struct CsName(pub Option<CsId>);

impl TexPrint for CsName {
    fn print_to(&self, st: &mut EngineState) {
        sprint_cs(st, self.0);
    }
}

/// `T` on a fresh line, as `print_nl` prints it.
#[derive(Copy, Clone)]
pub(crate) struct Nl<T>(pub T);

impl<T: TexPrint> TexPrint for Nl<T> {
    fn print_to(&self, st: &mut EngineState) {
        print_nl(st, &self.0);
    }
}

/// The family font command of a math size: `\textfont`, `\scriptfont` or `\scriptscriptfont`.
#[derive(Copy, Clone)]
pub(crate) struct FamFont(pub i32);

impl TexPrint for FamFont {
    fn print_to(&self, st: &mut EngineState) {
        print_size(st, self.0);
    }
}

/// `' in <mode>`, closing a `You can't use` message.
#[derive(Copy, Clone)]
pub(crate) struct InMode(pub crate::mode::ModeName);

impl TexPrint for InMode {
    fn print_to(&self, st: &mut EngineState) {
        print_in_mode(st, self.0);
    }
}

/// A control sequence name, printed after `\escapechar`.
#[derive(Copy, Clone)]
pub(crate) struct Esc<T>(pub T);

impl<T: TexPrint> TexPrint for Esc<T> {
    #[inline]
    fn print_to(&self, st: &mut EngineState) {
        print_escape_char(st);
        self.0.print_to(st);
    }
}

macro_rules! tex_print_tuple {
    ($($part:ident: $T:ident),+) => {
        impl<$($T: TexPrint),+> TexPrint for ($($T,)+) {
            #[inline]
            fn print_to(&self, st: &mut EngineState) {
                let ($($part,)+) = self;
                $($part.print_to(st);)+
            }
        }
    };
}

tex_print_tuple!(a: A);
tex_print_tuple!(a: A, b: B);
tex_print_tuple!(a: A, b: B, c: C);
tex_print_tuple!(a: A, b: B, c: C, d: D);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
tex_print_tuple!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L);

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

pub(crate) fn print_nl(st: &mut EngineState, s: impl TexPrint) {
    if st.shipout.selector < Selector::NoPrint || !at_line_start(st) {
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

pub(crate) fn print_esc(st: &mut EngineState, s: impl TexPrint) {
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
    st.diagnostics.mirror(c as u8);
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

pub(crate) fn print(st: &mut EngineState, s: impl TexPrint) {
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

/// TeX's `print_err`: prints `message` after the error prefix, capturing it as
/// a `kind` diagnostic that the caller reports.
pub(crate) fn print_err(st: &mut EngineState, kind: DiagnosticKind, message: impl TexPrint) {
    print_error_prefix(st);
    begin_capture(st, kind);
    print(st, message);
}

/// TeX Live's `print_ignored_err`: an error that `\ignoreprimitiveerror` turns into a warning.
pub(crate) fn print_ignored_err(st: &mut EngineState, message: &str) {
    if st.config.file_line_error_style_p != 0 {
        print_file_line(st);
    } else {
        print_nl(st, "");
    }
    begin_warning(st);
    print(st, "ignored error: ");
    print(st, message);
    end_capture(st);
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

fn print_write_whatsit(st: &mut EngineState, s: &str, v: i32) {
    print_esc(st, s);
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
        let units: Vec<u16> = st
            .strings
            .str_range(s)
            .map(|j| st.strings.str_pool[j])
            .collect();
        let mut units = units.into_iter().peekable();
        while let Some(unit) = units.next() {
            let c = i32::from(unit);
            if c == quote_char {
                print(st, StrNumber::single_char(quote_char));
                quote_char = QUOTE_SUM - quote_char;
                print(st, StrNumber::single_char(quote_char));
            }
            let low = if (0xD800..=0xDBFF).contains(&unit) {
                units.next_if(|lo| (0xDC00..=0xDFFF).contains(lo))
            } else {
                None
            };
            match low {
                Some(lo) => print_char(st, 0x10000 + (c - 0xD800) * 0x400 + i32::from(lo) - 0xDC00),
                None => print(st, StrNumber::single_char(c)),
            }
        }
    }

    if quote_char != 0 {
        print_char(st, quote_char);
    }
}

/// The name string of a control sequence from the first hash bucket on.
fn cs_text(st: &EngineState, p: CsId) -> StrNumber {
    match p.kind() {
        CsKind::Primitive(prim) => st.eqtb.prim_text(prim).unwrap_or(StrNumber::from_raw(-1)),
        _ => st.eqtb.text(p),
    }
}

/// Prints `\IMPOSSIBLE.` for no control sequence, as TeX does for 0.
pub(crate) fn print_cs(st: &mut EngineState, p: Option<CsId>) {
    let Some(p) = p else {
        return print_esc(st, "IMPOSSIBLE.");
    };
    match p.kind() {
        CsKind::Active(c) => print_char(st, c.raw()),
        CsKind::Single(c) => {
            let c = i32::from(c);
            print_esc(st, StrNumber::single_char(c));
            if st.eqtb.cat_code(c) == Cmd::Letter {
                print_char(st, b' ');
            }
        }
        CsKind::Null => {
            print_esc(st, "csname");
            print_esc(st, "endcsname");
            print_char(st, b' ');
        }
        CsKind::Undefined => print_esc(st, "IMPOSSIBLE."),
        _ if p > st.eqtb.cs_top => print_esc(st, "IMPOSSIBLE."),
        _ if st.eqtb.text(p) >= st.strings.str_ptr => print_esc(st, "NONEXISTENT."),
        _ => {
            print_esc(st, cs_text(st, p));
            print_char(st, b' ');
        }
    }
}

/// Like [`print_cs`] but without the trailing space.
pub(crate) fn sprint_cs(st: &mut EngineState, p: Option<CsId>) {
    let Some(p) = p else {
        return print_esc(st, "IMPOSSIBLE.");
    };
    match p.kind() {
        CsKind::Active(c) => print_char(st, c.raw()),
        CsKind::Single(c) => print_esc(st, StrNumber::single_char(i32::from(c))),
        CsKind::Null => {
            print_esc(st, "csname");
            print_esc(st, "endcsname");
        }
        _ => print_esc(st, cs_text(st, p)),
    }
}

fn print_native_word(st: &mut EngineState, p: NodePtr<NativeWordNode>) {
    let len = st.mem.entries(p).len();
    let mut i = 0;
    while i < len {
        let c = st.mem.entries(p)[i];
        if i + 1 < len {
            if let Some(pair) = surrogate_pair(c, st.mem.entries(p)[i + 1]) {
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

fn print_sa_num(st: &mut EngineState, q: NodePtr<SaElement>) {
    let mem = &st.mem;
    let n = match q.word(mem) {
        Some(w) => Some(mem[w].sa_num),
        None => {
            let q1 = mem[q].link;
            let q2 = q1.and_then(|q1| mem[q1].link);
            let q3 = q2.and_then(|q2| mem[q2].link);
            q1.zip(q2).zip(q3).map(|((q1, q2), q3)| {
                let digit = |i: u16| i32::from(i);
                digit(mem[q].sa_index) % 64
                    + 64 * digit(mem[q1].sa_index)
                    + 64 * 64 * (digit(mem[q2].sa_index) + 64 * digit(mem[q3].sa_index))
            })
        }
    };
    if let Some(n) = n {
        print_int(st, n);
    }
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

pub(crate) fn print_mode(st: &mut EngineState, m: crate::mode::ModeName) {
    print(st, m.as_str());
}

pub(crate) fn print_in_mode(st: &mut EngineState, m: crate::mode::ModeName) {
    print(st, "' in ");
    print(st, m.as_str());
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
    let line_at = st.save_stack.boundary().map_or(0, |b| b.2);
    if line_at != 0 {
        if e {
            print(st, " entered at line ");
        } else {
            print(st, " at line ");
        }
        print_int(st, line_at);
    }
}

pub(crate) fn token_show(st: &mut EngineState, list: TokenList) {
    if !list.is_null() {
        let from = st.mem.tokens.cursor(list);
        show_token_list(st, from, TokCursor::END, 10_000_000);
    }
}

pub(crate) fn print_spec(st: &mut EngineState, p: GlueSpecId, s: Option<&str>) {
    let spec = st.mem.glue.contains(p.raw()).then_some(p);
    let Some(gs) = spec else {
        print_char(st, b'*');
        return;
    };
    print_scaled(st, st.mem[gs].width);
    if let Some(s) = s {
        print(st, s);
    }
    if st.mem[gs].stretch != Scaled::ZERO {
        print(st, " plus ");
        print_glue(st, st.mem[gs].stretch, st.mem[gs].stretch_order, s);
    }
    if st.mem[gs].shrink != Scaled::ZERO {
        print(st, " minus ");
        print_glue(st, st.mem[gs].shrink, st.mem[gs].shrink_order, s);
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
        print(st, StrNumber::single_char(c));
    } else {
        print_char(st, c);
    }
}

fn print_fam_and_char(st: &mut EngineState, p: MathWord) {
    print_esc(st, "fam");
    print_int(st, p.fam());
    print_char(st, b' ');
    print_code(st, p.full_char());
}

fn print_delimiter(st: &mut EngineState, p: DelimiterWord) {
    let (small_fam, small_char) = p.small();
    let (large_fam, large_char) = p.large();
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
    print(st, StrNumber::single_char(i32::from(c)));
}

fn print_font_and_char(st: &mut EngineState, p: MemPtr) {
    match p.filter(|&q| st.mem.contains(Some(q))) {
        None => print_esc(st, "CLOBBERED."),
        Some(q) => {
            let ch = NodePtr::<CharNode>::at(q);
            print_font_char(st, st.mem[ch].font, st.mem[ch].character);
        }
    }
}

fn print_mark(st: &mut EngineState, list: TokenList) {
    print_char(st, b'{');
    if !st.mem.tokens.holds_list(list) {
        print_esc(st, "CLOBBERED.");
    } else {
        let from = st.mem.tokens.cursor(list);
        let l = st.config.max_print_line - 10;
        show_token_list(st, from, TokCursor::END, l);
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

/// Prints `bytes` with `print_raw_char`, one column each.
pub(crate) fn print_raw_bytes(st: &mut EngineState, bytes: &[u8]) {
    for &b in bytes {
        print_raw_char(st, i32::from(b), true);
    }
}
