//! Native evaluation of pgf's `\pgfmathparse`, in floating point.
//!
//! `\pgfmathparse` opens a group, expands its argument into
//! `\pgfmath@expression` and hands it to `\pgfmathparse@trynumber@loop`. On
//! that first call we evaluate the expression here, define `\pgfmathresult`
//! and redirect the call to `\pgfmath@parse@end`, which takes the same
//! argument and does pgf's own post-processing and group exit. With the FPU
//! installed the same path runs with float-format results, and the FPU's
//! most used entry points are answered once `macro_call` has read their
//! arguments.
//!
//! Results are computed in `f64`, like pgf's luamath library, so they may
//! differ from pgf's fixed-point macros in the last digits. Expressions using
//! anything [`eval`] does not implement fall back to the macros.
//!
//! `TECTONIC_PGFMATH=native|check|off` overrides the engine config.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::cmd::{
    ASSIGN_DIMEN, ASSIGN_INT, CALL, LEFT_BRACE, LETTER, MATH_SHIFT, OTHER_CHAR, REGISTER,
    RIGHT_BRACE, SET_FONT, SPACER, SUB_MARK, SUP_MARK, TAB_MARK,
};
use crate::codes::ValueLevel;
use crate::consts::{HASH_BASE, TEX_NULL};
use crate::errors::Abort;
use crate::node::{NodePtr, SaNode};
use crate::params::IntPar;
use crate::state::EngineState;
use crate::token::Token;
use crate::units::{FontId, MemPtr};

pub(crate) mod eval;

use eval::{Format, Func, Item};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Off,
    Native,
    Check,
}

const KIND_UNKNOWN: u8 = 0;
const KIND_NONE: u8 = 1;
const KIND_TRY: u8 = 2;
/// Plus the index into [`NATIVES`].
const KIND_NATIVE: u8 = 3;

/// What each control sequence is to us, by `eqtb` slot, so the check in
/// `macro_call` is one array read. Cleared on rewind, since a rewound slot
/// may later carry another name.
#[derive(Default)]
pub(crate) struct Kinds {
    table: Vec<u8>,
}

impl Kinds {
    /// Whether a call of `cs` needs [`intercept`].
    #[inline(always)]
    pub(crate) fn wanted(&self, cs: i32) -> bool {
        self.table.get(cs as usize) != Some(&KIND_NONE)
    }

    /// Whether a call of `cs` may be answered by [`native_macro`].
    #[inline(always)]
    pub(crate) fn native(&self, cs: i32) -> bool {
        self.table
            .get(cs as usize)
            .is_some_and(|&k| k >= KIND_NATIVE)
    }

    pub(crate) fn clear(&mut self) {
        self.table.clear();
    }
}

/// Tracing parameters whose output the macro path would produce.
const TRACING: &[IntPar] = &[
    IntPar::TracingMacros,
    IntPar::TracingCommands,
    IntPar::TracingAssigns,
    IntPar::TracingGroups,
    IntPar::TracingIfs,
    IntPar::TracingRestores,
    IntPar::TracingOnline,
];

/// Control sequences the evaluator reads or writes, by `eqtb` slot.
struct Names {
    parse_end: i32,
    is_number: i32,
    push_operand: i32,
    expression: i32,
    result: i32,
    units_declared: i32,
    if_true: i32,
    if_false: i32,
    relax: i32,
    let_: i32,
    edef: i32,
    quickparse: i32,
    postparse: i32,
    trig_format: i32,
    /// `\f@encoding`, `\f@family`, `\f@series`, `\f@shape`, `\f@size`.
    nfss: [i32; 5],
    /// Defined by the FPU library, which loads later than pgfmath.
    comparison: Option<i32>,
    fpu_push_operand: Option<i32>,
    fpu_is_number: Option<i32>,
}

fn resolve(st: &mut EngineState) -> Option<Names> {
    let mut nfss = [0; 5];
    for (slot, name) in
        nfss.iter_mut()
            .zip(["f@encoding", "f@family", "f@series", "f@shape", "f@size"])
    {
        *slot = lookup(st, name)?;
    }
    Some(Names {
        parse_end: lookup(st, "pgfmath@parse@end")?,
        is_number: lookup(st, "pgfmathparse@expression@is@number")?,
        push_operand: lookup(st, "pgfmath@stack@push@operand")?,
        expression: lookup(st, "pgfmath@expression")?,
        result: lookup(st, "pgfmathresult")?,
        units_declared: lookup(st, "ifpgfmathunitsdeclared")?,
        if_true: lookup(st, "iftrue")?,
        if_false: lookup(st, "iffalse")?,
        relax: lookup(st, "relax")?,
        let_: lookup(st, "let")?,
        edef: lookup(st, "edef")?,
        quickparse: lookup(st, "ifpgfmath@quickparse")?,
        postparse: lookup(st, "pgfmathpostparse")?,
        trig_format: lookup(st, "pgfmath@trig@format@choice")?,
        nfss,
        comparison: None,
        fpu_push_operand: None,
        fpu_is_number: None,
    })
}

impl Names {
    /// Look up the FPU's names once it has defined them.
    fn find_fpu(&mut self, st: &mut EngineState) {
        if self.comparison.is_none() {
            self.comparison = lookup(st, "ifpgfmathfloatcomparison");
        }
        if self.fpu_push_operand.is_none() {
            self.fpu_push_operand = lookup(st, "pgfmathfloat@stack@push@operand");
        }
        if self.fpu_is_number.is_none() {
            self.fpu_is_number = lookup(st, "pgfmathfloat@parse@expression@is@number");
        }
    }
}

#[derive(Default)]
struct Stats {
    native: u64,
    numbers: u64,
    natives: u64,
    fallback: std::collections::BTreeMap<&'static str, u64>,
    checked: u64,
    differed: u64,
    format_differed: u64,
    worst: f64,
}

/// What a native evaluation defines `\pgfmathresult` to.
enum Answer {
    /// `\let\pgfmathresult=\pgfmath@expression`, pgf's plain-number lane.
    Expression,
    /// Fresh text: a computed result, or a float-format plain number.
    Parsed(eval::Outcome),
}

struct Pending {
    answer: Answer,
    expression: String,
}

struct Pgfmath {
    /// Forced by `TECTONIC_PGFMATH`, else taken from the engine config.
    mode: Option<Mode>,
    /// Resolved names, dropped on rewind.
    names: Option<Names>,
    stats: Stats,
    /// Check mode: the native answer waiting for the macro run to finish.
    pending: Option<Pending>,
}

thread_local! {
    static STATE: RefCell<Pgfmath> = RefCell::new(Pgfmath {
        mode: match std::env::var("TECTONIC_PGFMATH").as_deref() {
            Ok("native") => Some(Mode::Native),
            Ok("check") => Some(Mode::Check),
            Ok("off") => Some(Mode::Off),
            _ => None,
        },
        names: None,
        stats: Stats::default(),
        pending: None,
    });
}

fn mode_for(state: &Pgfmath, st: &EngineState) -> Mode {
    state.mode.unwrap_or(if st.config.native_pgfmath {
        Mode::Native
    } else {
        Mode::Off
    })
}

/// Forget classifications and resolved names, e.g. after a rewind.
pub(crate) fn invalidate(st: &mut EngineState) {
    st.pgfmath.clear();
    STATE.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            state.names = None;
            state.pending = None;
        }
    });
}

/// Whether `cs` is named `name`, without allocating.
fn named(st: &EngineState, cs: i32, name: &str) -> bool {
    if cs < HASH_BASE {
        return false;
    }
    let text = st.eqtb.text(cs);
    if text.0 <= 0 || text.0 >= st.strings.str_ptr {
        return false;
    }
    let start = st.strings.str_start[text.slot()] as usize;
    let end = st.strings.str_start[text.next().slot()] as usize;
    let units = &st.strings.str_pool[start..end];
    units.len() == name.len()
        && units
            .iter()
            .zip(name.bytes())
            .all(|(&u, b)| u == u16::from(b))
}

fn classify(st: &EngineState, cs: i32) -> u8 {
    if named(st, cs, "pgfmathparse@trynumber@loop") {
        return KIND_TRY;
    }
    NATIVES
        .iter()
        .position(|&(name, _)| named(st, cs, name))
        .map_or(KIND_NONE, |i| KIND_NATIVE + i as u8)
}

/// Called at the top of `macro_call` when [`Kinds::wanted`] says so.
/// Returns the control sequence to expand instead, when the call starts a
/// `\pgfmathparse` we answered natively.
#[cold]
#[inline(never)]
pub(crate) fn intercept(st: &mut EngineState) -> Option<i32> {
    let cs = st.scanner.cur_cs;
    let mut kind = st
        .pgfmath
        .table
        .get(cs as usize)
        .copied()
        .unwrap_or(KIND_UNKNOWN);
    let mode = STATE.with(|cell| mode_for(&cell.borrow(), st));
    if kind == KIND_UNKNOWN {
        kind = classify(st, cs);
        if let Ok(slot) = usize::try_from(cs) {
            if slot >= st.pgfmath.table.len() {
                st.pgfmath.table.resize(slot + 1, KIND_UNKNOWN);
            }
            // Check mode watches every call for pgf's own exit, so keeps asking.
            if mode != Mode::Check || kind != KIND_NONE {
                st.pgfmath.table[slot] = if mode == Mode::Off { KIND_NONE } else { kind };
            }
        }
    }
    if mode == Mode::Off {
        return None;
    }
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Pgfmath {
            names,
            stats,
            pending,
            ..
        } = &mut *state;
        if kind != KIND_TRY && pending.is_none() {
            return None;
        }
        if names.is_none() {
            *names = resolve(st);
        }
        let names = names.as_mut()?;

        let body = st.scanner.cur_chr;
        if mode == Mode::Check {
            if let Some(p) = pending.take() {
                if body == st.eqtb.equiv(names.parse_end) {
                    compare(st, names, stats, &p, false);
                    return None;
                }
                if body == st.eqtb.equiv(names.is_number) {
                    compare(st, names, stats, &p, true);
                    return None;
                }
                *pending = Some(p);
            }
        }
        if kind != KIND_TRY {
            return None;
        }
        let mut fail = |reason: &'static str| {
            *stats.fallback.entry(reason).or_default() += 1;
        };
        if TRACING.iter().any(|&p| st.eqtb.int_par(p) > 0)
            || st.eqtb.int_par(IntPar::GlobalDefs) != 0
        {
            fail("tracing or globaldefs");
            return None;
        }
        if !same_meaning(st, names.quickparse, names.if_false) {
            fail("quick parse");
            return None;
        }
        names.find_fpu(st);
        let format = match names.fpu_push_operand {
            Some(fpu) if same_meaning(st, names.push_operand, fpu) => Format::Float,
            _ => Format::Fixed,
        };
        let float_numbers = names
            .fpu_is_number
            .is_some_and(|fpu| same_meaning(st, names.is_number, fpu));

        let tokens = match expression_tokens(st, names.expression) {
            Ok(tokens) => tokens,
            Err(reason) => {
                fail(reason);
                return None;
            }
        };
        let answer = if is_number(&tokens) {
            if !same_meaning(st, names.postparse, names.relax) {
                fail("postparse on a plain number");
                return None;
            }
            if !float_numbers {
                Answer::Expression
            } else {
                let text: Option<Vec<u8>> =
                    tokens.iter().map(|t| u8::try_from(t.chr()).ok()).collect();
                match text.as_deref().and_then(eval::parse_number) {
                    Some(v) if v.is_finite() => Answer::Parsed(eval::Outcome {
                        result: eval::float(v),
                        units: false,
                    }),
                    _ => {
                        fail("malformed number");
                        return None;
                    }
                }
            }
        } else {
            let Some(font) = selected_font(st, names) else {
                fail("selectfont would load a font");
                return None;
            };
            let items = match expression_items(st, &tokens) {
                Ok(items) => items,
                Err(reason) => {
                    trace(st, &tokens, reason);
                    fail(reason);
                    return None;
                }
            };
            let mut constants = HashMap::new();
            for name in identifiers(&items) {
                if Func::from_name(&name).is_none() {
                    if let Some(text) = pseudo_constant(st, names, &name) {
                        constants.insert(name, text);
                    }
                }
            }
            let radians = macro_text(st, names.trig_format).as_deref() == Some(b"1");
            let constant = |name: &[u8]| constants.get(name).cloned();
            let env = eval::Env {
                font,
                radians,
                format,
                constant: &constant,
            };
            match eval::evaluate(&items, &env) {
                Ok(outcome) => Answer::Parsed(outcome),
                Err(reason) => {
                    trace(st, &tokens, reason);
                    fail(reason);
                    return None;
                }
            }
        };

        if mode == Mode::Check {
            *pending = Some(Pending {
                answer,
                expression: describe(st, &tokens),
            });
            return None;
        }
        match commit(st, names, &answer) {
            Ok(()) => {
                match answer {
                    Answer::Expression => stats.numbers += 1,
                    Answer::Parsed(_) => stats.native += 1,
                }
                Some(names.parse_end)
            }
            Err(_) => {
                fail("commit failed");
                None
            }
        }
    })
}

fn lookup(st: &mut EngineState, name: &str) -> Option<i32> {
    let buffer: Vec<i32> = name.chars().map(|c| c as i32).collect();
    let saved = st.eqtb.no_new_control_sequence;
    st.eqtb.no_new_control_sequence = true;
    let found = st
        .eqtb
        .id_lookup(&mut st.strings, &buffer, 0, buffer.len() as i32);
    st.eqtb.no_new_control_sequence = saved;
    match found {
        Ok(p) if p >= HASH_BASE && st.eqtb.text(p).0 > 0 => Some(p),
        _ => None,
    }
}

fn cs_name(st: &EngineState, cs: i32) -> Option<String> {
    if cs < HASH_BASE {
        return None;
    }
    let text = st.eqtb.text(cs);
    if text.0 <= 0 || text.0 >= st.strings.str_ptr {
        return None;
    }
    crate::introspect::read_string(st, text)
}

/// The tokens of a parameterless macro's body.
fn macro_tokens(st: &EngineState, cs: i32) -> Option<Vec<Token>> {
    if st.eqtb.eq_type(cs) != CALL {
        return None;
    }
    let mut p = MemPtr(st.eqtb.equiv(cs)).link(&st.mem);
    if p.is_null() || p.token(&st.mem) != Token::END_MATCH {
        return None;
    }
    p = p.link(&st.mem);
    let mut out = Vec::new();
    while !p.is_null() {
        out.push(p.token(&st.mem));
        p = p.link(&st.mem);
    }
    Some(out)
}

/// A parameterless macro's body, if it is plain ASCII characters.
fn macro_text(st: &EngineState, cs: i32) -> Option<Vec<u8>> {
    macro_tokens(st, cs)?
        .into_iter()
        .map(|t| (!t.is_cs() && t.chr() < 128).then_some(t.chr() as u8))
        .collect()
}

fn same_meaning(st: &EngineState, a: i32, b: i32) -> bool {
    st.eqtb.eq_type(a) == st.eqtb.eq_type(b) && st.eqtb.equiv(a) == st.eqtb.equiv(b)
}

/// Names that could be pgfmath functions or constants.
fn identifiers(items: &[(Item, bool)]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    for &(item, space) in items {
        let c = match item {
            Item::Char(c, _) => Some(c),
            _ => None,
        };
        let continues = !current.is_empty()
            && !space
            && c.is_some_and(|c| c == b'_' || c.is_ascii_alphanumeric());
        if !continues && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        if continues || c.is_some_and(|c| c == b'_' || c.is_ascii_alphabetic()) {
            current.push(c.expect("char"));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The value of a zero-argument function whose body is
/// `\let\pgfmathresult=\macro` or `\edef\pgfmathresult{\macro}`, as pgfplots
/// declares `x` and `y`.
fn pseudo_constant(st: &mut EngineState, names: &Names, name: &[u8]) -> Option<Vec<u8>> {
    let name = std::str::from_utf8(name).ok()?;
    let arity = lookup(st, &format!("pgfmath@operation@{name}@arity"))?;
    if macro_text(st, arity).as_deref() != Some(b"0") {
        return None;
    }
    let function = lookup(st, &format!("pgfmath{name}@"))?;
    let body = macro_tokens(st, function)?;
    let is = |t: &Token, cs: i32| t.is_cs() && same_meaning(st, t.cs(), cs);
    let result = |t: &Token| t.is_cs() && t.cs() == names.result;
    let char_is = |t: &Token, c: u8| !t.is_cs() && t.chr() == i32::from(c);
    let target = match body.as_slice() {
        [l, r, eq, t] if is(l, names.let_) && result(r) && char_is(eq, b'=') && t.is_cs() => t.cs(),
        [l, r, t] if is(l, names.let_) && result(r) && t.is_cs() => t.cs(),
        [d, r, open, t, close]
            if is(d, names.edef)
                && result(r)
                && char_is(open, b'{')
                && t.is_cs()
                && char_is(close, b'}') =>
        {
            t.cs()
        }
        _ => return None,
    };
    expanded_text(st, target)
}

/// A parameterless macro's text, following bodies that are a single macro.
fn expanded_text(st: &EngineState, mut cs: i32) -> Option<Vec<u8>> {
    for _ in 0..8 {
        let tokens = macro_tokens(st, cs)?;
        match tokens.as_slice() {
            [t] if t.is_cs() => cs = t.cs(),
            _ => return macro_text(st, cs),
        }
    }
    None
}

/// The body of `\pgfmath@expression`.
fn expression_tokens(st: &EngineState, expression: i32) -> Result<Vec<Token>, &'static str> {
    macro_tokens(st, expression).ok_or("expression is not a plain macro")
}

/// Whether pgf's `\pgfmathparse@trynumber@loop` takes its fast lane.
fn is_number(tokens: &[Token]) -> bool {
    tokens.iter().all(|t| {
        !t.is_cs()
            && !matches!(t.cmd(), LEFT_BRACE | RIGHT_BRACE | SPACER)
            && (t.chr() == i32::from(b'.')
                || (i32::from(b'0')..=i32::from(b'9')).contains(&t.chr()))
    })
}

/// The expression as evaluator items, each with whether a space preceded it.
fn expression_items(st: &EngineState, tokens: &[Token]) -> Result<Vec<(Item, bool)>, &'static str> {
    let mut body = tokens;
    // \pgfmath@bgroup@strip: a lone outer group loses its braces.
    while body
        .first()
        .is_some_and(|t| !t.is_cs() && t.cmd() == LEFT_BRACE)
    {
        let mut depth = 0;
        let mut close = None;
        for (i, t) in body.iter().enumerate() {
            if t.is_cs() {
                continue;
            }
            match t.cmd() {
                LEFT_BRACE => depth += 1,
                RIGHT_BRACE => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if close != Some(body.len() - 1) {
            return Err("braces");
        }
        body = &body[1..body.len() - 1];
    }
    let mut items = Vec::with_capacity(body.len());
    let mut space = false;
    for &tok in body {
        if tok.is_cs() {
            items.push((register_item(st, tok.cs())?, space));
            space = false;
            continue;
        }
        let (cmd, chr) = (tok.cmd(), tok.chr());
        match cmd {
            SPACER => space = true,
            LETTER | OTHER_CHAR | SUP_MARK | SUB_MARK | TAB_MARK | MATH_SHIFT if chr < 128 => {
                items.push((Item::Char(chr as u8, cmd == OTHER_CHAR), space));
                space = false;
            }
            _ => return Err("unsupported token"),
        }
    }
    Ok(items)
}

fn register_item(st: &EngineState, cs: i32) -> Result<Item, &'static str> {
    if cs < HASH_BASE {
        return Err("active or single-character control sequence");
    }
    let m = st.eqtb.equiv(cs);
    match st.eqtb.eq_type(cs) {
        ASSIGN_DIMEN => Ok(Item::Dimen(st.eqtb.int_at(m))),
        ASSIGN_INT => Ok(Item::Count(st.eqtb.int_at(m))),
        REGISTER if ValueLevel::from_raw(m).is_none() => {
            let sa = NodePtr::<SaNode>::at(m);
            match sa.sa_type(&st.mem) {
                ValueLevel::Int => Ok(Item::Count(sa.sa_int(&st.mem))),
                ValueLevel::Dimen => Ok(Item::Dimen(sa.sa_int(&st.mem))),
                _ => Err("unsupported register"),
            }
        }
        _ => Err("unsupported control sequence"),
    }
}

/// The font `\pgfmath@selectfont` (LaTeX's `\selectfont`) picks, if it is
/// already loaded. If not, the macros would load it, so we fall back.
fn selected_font(st: &mut EngineState, names: &Names) -> Option<eval::Font> {
    let mut name = String::new();
    for (i, &cs) in names.nfss.iter().enumerate() {
        if i > 0 {
            name.push('/');
        }
        name.push_str(std::str::from_utf8(&macro_text(st, cs)?).ok()?);
    }
    let id = lookup(st, &name)?;
    if st.eqtb.eq_type(id) != SET_FONT {
        return None;
    }
    let font = FontId(st.eqtb.equiv(id));
    Some(eval::Font {
        quad: st.fonts.param(font, crate::fonts::QUAD_CODE).0,
        x_height: st.fonts.param(font, crate::fonts::X_HEIGHT_CODE).0,
    })
}

/// A fresh parameterless macro body. Letters get catcode 11 like the `Y`
/// and `e` of `\pgfmathfloatcreate`, everything else catcode 12.
fn result_macro(st: &mut EngineState, text: &[u8]) -> Result<MemPtr, Abort> {
    let head = st.mem.get_avail()?;
    head.set_info(&mut st.mem, TEX_NULL);
    let mut tail = head;
    st.mem.store_new_token(&mut tail, Token::END_MATCH)?;
    for &b in text {
        let tok = if b.is_ascii_alphabetic() {
            Token::letter(b)
        } else {
            Token::other(b)
        };
        st.mem.store_new_token(&mut tail, tok)?;
    }
    Ok(head)
}

/// Make the definitions the macro run would have left behind.
fn commit(st: &mut EngineState, names: &Names, answer: &Answer) -> Result<(), Abort> {
    let body = match answer {
        Answer::Expression => {
            let shared = MemPtr(st.eqtb.equiv(names.expression));
            st.mem.add_token_ref(shared);
            shared
        }
        Answer::Parsed(outcome) => {
            if outcome.units {
                let (t, e) = (st.eqtb.eq_type(names.if_true), st.eqtb.equiv(names.if_true));
                st.eqtb
                    .geq_define(&mut st.mem, &mut st.scanner, names.units_declared, t, e)?;
            }
            result_macro(st, &outcome.result)?
        }
    };
    st.eqtb.eq_define(
        &mut st.save_stack,
        &mut st.mem,
        &mut st.scanner,
        names.result,
        CALL,
        body.0,
    )
}

/// Check mode: compare the macro run's `\pgfmathresult` with ours.
fn compare(st: &EngineState, names: &Names, stats: &mut Stats, pending: &Pending, number: bool) {
    stats.checked += 1;
    let (ours, units) = match &pending.answer {
        Answer::Expression => {
            if !number {
                stats.format_differed += 1;
            }
            return;
        }
        Answer::Parsed(_) if number => return,
        Answer::Parsed(outcome) => (&outcome.result, outcome.units),
    };
    let Some(theirs) = macro_text(st, names.result) else {
        return;
    };
    let shape = |t: &[u8]| (t.contains(&b'Y'), t.contains(&b'.'));
    if shape(&theirs) != shape(ours)
        || units != same_meaning(st, names.units_declared, names.if_true)
    {
        stats.format_differed += 1;
        if stats.format_differed <= 20 {
            eprintln!(
                "pgfmath format: `{}` native {} macro {}",
                pending.expression,
                String::from_utf8_lossy(ours),
                String::from_utf8_lossy(&theirs)
            );
        }
    }
    let (Some(a), Some(b)) = (eval::parse_number(ours), eval::parse_number(&theirs)) else {
        return;
    };
    let error = (a - b).abs() / b.abs().max(1.0);
    if error > 1e-3 {
        stats.differed += 1;
        if stats.differed <= 20 {
            eprintln!(
                "pgfmath differs: `{}` native {} macro {}",
                pending.expression,
                String::from_utf8_lossy(ours),
                String::from_utf8_lossy(&theirs)
            );
        }
    }
    stats.worst = stats.worst.max(error);
}

fn describe(st: &EngineState, tokens: &[Token]) -> String {
    let mut text = String::new();
    for tok in tokens {
        if tok.is_cs() {
            text.push('\\');
            text.push_str(&cs_name(st, tok.cs()).unwrap_or_else(|| format!("<{}>", tok.cs())));
            text.push(' ');
        } else {
            text.push(char::from_u32(tok.chr() as u32).unwrap_or('?'));
        }
    }
    text
}

fn trace(st: &EngineState, tokens: &[Token], reason: &str) {
    let Ok(filter) = std::env::var("TECTONIC_PGFMATH_TRACE") else {
        return;
    };
    if reason.contains(filter.as_str()) {
        eprintln!("pgfmath fallback ({reason}): {}", describe(st, tokens));
    }
}

// ---- FPU and pgfplots macros ---------------------------------------------

/// A macro answered natively once `macro_call` has read its arguments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Native {
    ParseNumber,
    LessThan,
    Max,
    Min,
    ToFixed,
    Shift,
    Flags,
}

const NATIVES: &[(&str, Native)] = &[
    ("pgfmathfloatparsenumber", Native::ParseNumber),
    ("pgfmathfloatqparsenumber", Native::ParseNumber),
    ("pgfmathfloatlessthan@", Native::LessThan),
    ("pgfmathfloatlessthan", Native::LessThan),
    ("pgfplotsmathfloatmax", Native::Max),
    ("pgfplotsmathfloatmin", Native::Min),
    ("pgfmathfloattofixed@", Native::ToFixed),
    ("pgfmathfloattofixed", Native::ToFixed),
    ("pgfmathfloatshift@", Native::Shift),
    ("pgfmathfloatshift", Native::Shift),
    ("pgfmathfloatgetflagstomacro", Native::Flags),
];

/// What a native macro leaves behind.
enum Effect {
    /// A parameterless macro, `\def` or `\gdef`.
    Define { cs: i32, text: Vec<u8> },
    /// `\global\pgfmathfloatcomparisontrue` or `false`.
    Comparison(bool),
}

/// The tokens of a macro argument list.
fn list_tokens(st: &EngineState, mut p: MemPtr) -> Vec<Token> {
    let mut out = Vec::new();
    while !p.is_null() {
        out.push(p.token(&st.mem));
        p = p.link(&st.mem);
    }
    out
}

/// A parameterless, unprotected macro's body.
fn simple_body(st: &EngineState, cs: i32) -> Option<Vec<Token>> {
    if cs < HASH_BASE
        || !matches!(
            st.eqtb.eq_type(cs),
            crate::cmd::CALL | crate::cmd::LONG_CALL
        )
    {
        return None;
    }
    let first = MemPtr(st.eqtb.equiv(cs)).link(&st.mem);
    if first.is_null() || first.token(&st.mem) != Token::END_MATCH {
        return None;
    }
    Some(list_tokens(st, first.link(&st.mem)))
}

fn chars(tokens: &[Token]) -> Option<Vec<u8>> {
    tokens
        .iter()
        .map(|t| {
            (!t.is_cs() && t.chr() < 128 && !matches!(t.cmd(), LEFT_BRACE | RIGHT_BRACE))
                .then_some(t.chr() as u8)
        })
        .collect()
}

/// `\edef` of `tokens` when it only expands parameterless macros.
fn expand_full(st: &EngineState, tokens: &[Token], depth: u32) -> Option<Vec<u8>> {
    if depth > 16 {
        return None;
    }
    let mut out = Vec::new();
    for t in tokens {
        if t.is_cs() {
            out.extend(expand_full(st, &simple_body(st, t.cs())?, depth + 1)?);
        } else {
            out.extend(chars(std::slice::from_ref(t))?);
        }
    }
    Some(out)
}

/// `\expandafter` once over the first token, then plain characters.
fn expand_once(st: &EngineState, tokens: &[Token]) -> Option<Vec<u8>> {
    match tokens.split_first() {
        Some((first, rest)) if first.is_cs() => {
            let mut all = simple_body(st, first.cs())?;
            all.extend_from_slice(rest);
            chars(&all)
        }
        _ => chars(tokens),
    }
}

/// `<flags>Y<mantissa>e<exponent>]` split into its parts.
fn split_float(text: &[u8]) -> Option<(u8, &[u8], i32)> {
    if text.len() < 5 || text[1] != b'Y' || *text.last()? != b']' || !text[0].is_ascii_digit() {
        return None;
    }
    let body = &text[2..text.len() - 1];
    let e = body.iter().rposition(|&c| c == b'e')?;
    let exponent = std::str::from_utf8(&body[e + 1..]).ok()?.parse().ok()?;
    Some((text[0], &body[..e], exponent))
}

/// `\pgfmathfloattofixed`, which shifts the mantissa's digits as text.
fn to_fixed(float: &[u8]) -> Option<Vec<u8>> {
    let (flags, mantissa, e) = split_float(float)?;
    let magnitude = |mantissa: &[u8]| -> Option<Vec<u8>> {
        let dot = mantissa.iter().position(|&c| c == b'.')?;
        let (int, frac) = (&mantissa[..dot], &mantissa[dot + 1..]);
        let mut out = Vec::new();
        if e < 0 {
            out.extend_from_slice(b"0.");
            out.extend(std::iter::repeat_n(b'0', (-e - 1) as usize));
            out.extend_from_slice(int);
            if frac != b"0" {
                out.extend_from_slice(frac);
            }
            return Some(out);
        }
        let mut digits = frac.to_vec();
        digits.extend_from_slice(b"000000000");
        let e = e as usize;
        out.extend_from_slice(int);
        if e <= 8 {
            out.extend_from_slice(&digits[..e]);
            out.push(b'.');
            out.extend_from_slice(&digits[e..]);
            return Some(out);
        }
        out.extend_from_slice(&digits[..8]);
        let mut rest = &digits[8..];
        let mut e = e - 8;
        loop {
            out.push(rest[0]);
            rest = &rest[1..];
            e -= 1;
            if rest.is_empty() {
                out.extend(std::iter::repeat_n(b'0', e));
                out.extend_from_slice(b".0");
                return Some(out);
            }
            if e == 0 {
                out.push(b'.');
                out.extend_from_slice(rest);
                return Some(out);
            }
        }
    };
    match flags {
        b'0' => Some(b"0.0".to_vec()),
        b'1' => magnitude(mantissa),
        b'2' => {
            let mut out = vec![b'-'];
            out.extend(magnitude(mantissa)?);
            Some(out)
        }
        b'3' => Some(b"nan".to_vec()),
        b'4' => Some(b"inf".to_vec()),
        b'5' => Some(b"-inf".to_vec()),
        _ => None,
    }
}

/// A number as `\pgfmathfloatparsenumber` accepts it.
fn parse_float_input(text: &[u8]) -> Option<f64> {
    match text.to_ascii_lowercase().as_slice() {
        b"nan" | b"-nan" => Some(f64::NAN),
        b"inf" | b"infty" | b"+inf" | b"+infty" => Some(f64::INFINITY),
        b"-inf" | b"-infty" => Some(f64::NEG_INFINITY),
        _ => eval::parse_number(text),
    }
}

fn run_native(
    st: &EngineState,
    names: &Names,
    kind: Native,
    args: &[Vec<Token>],
) -> Option<Vec<Effect>> {
    let result = |text: Vec<u8>| Effect::Define {
        cs: names.result,
        text,
    };
    let compare = |a: &[Token], b: &[Token]| -> Option<(bool, Vec<u8>, Vec<u8>)> {
        let (ta, tb) = (expand_full(st, a, 0)?, expand_full(st, b, 0)?);
        split_float(&ta)?;
        split_float(&tb)?;
        let (va, vb) = (eval::parse_number(&ta)?, eval::parse_number(&tb)?);
        Some((va < vb, ta, tb))
    };
    Some(match (kind, args) {
        (Native::ParseNumber, [a]) => {
            let v = parse_float_input(&expand_full(st, a, 0)?)?;
            vec![result(eval::float(v))]
        }
        (Native::LessThan, [a, b]) => {
            let (less, _, _) = compare(a, b)?;
            names.comparison?;
            vec![
                Effect::Comparison(less),
                result(if less {
                    b"1.0".to_vec()
                } else {
                    b"0.0".to_vec()
                }),
            ]
        }
        (Native::Max | Native::Min, [a, b]) => {
            let (less, ta, tb) = compare(a, b)?;
            names.comparison?;
            let pick = if (kind == Native::Max) == less {
                tb
            } else {
                ta
            };
            vec![Effect::Comparison(less), result(pick)]
        }
        (Native::ToFixed, [a]) => vec![result(to_fixed(&expand_once(st, a)?)?)],
        (Native::Shift, [a, b]) => {
            let text = expand_once(st, a)?;
            let (flags, mantissa, e) = split_float(&text)?;
            let by: i32 = std::str::from_utf8(&chars(b)?).ok()?.trim().parse().ok()?;
            let mut out = vec![flags, b'Y'];
            out.extend_from_slice(mantissa);
            out.extend(format!("e{}]", e.checked_add(by)?).bytes());
            vec![result(out)]
        }
        (Native::Flags, [a, target]) => {
            let text = expand_once(st, a)?;
            let (flags, _, _) = split_float(&text)?;
            match target.as_slice() {
                [t] if t.is_cs() && t.cs() >= HASH_BASE => vec![Effect::Define {
                    cs: t.cs(),
                    text: vec![flags],
                }],
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// Called by `macro_call` once it has read the arguments of a macro that
/// [`Kinds::native`] marks. Returns true when the call was answered natively,
/// in which case the caller frees the arguments and skips the body.
#[cold]
#[inline(never)]
pub(crate) fn native_macro(st: &mut EngineState, args: &[MemPtr]) -> bool {
    let cs = st.scanner.warning_index;
    let Some(kind) = st
        .pgfmath
        .table
        .get(cs as usize)
        .and_then(|k| k.checked_sub(KIND_NATIVE))
        .map(|i| NATIVES[usize::from(i)].1)
    else {
        return false;
    };
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if mode_for(&state, st) != Mode::Native
            || TRACING.iter().any(|&p| st.eqtb.int_par(p) > 0)
            || st.eqtb.int_par(IntPar::GlobalDefs) != 0
        {
            return false;
        }
        if state.names.is_none() {
            state.names = resolve(st);
        }
        let Pgfmath { names, stats, .. } = &mut *state;
        let Some(names) = names.as_mut() else {
            return false;
        };
        names.find_fpu(st);
        let args: Vec<Vec<Token>> = args.iter().map(|&p| list_tokens(st, p)).collect();
        let Some(effects) = run_native(st, names, kind, &args) else {
            *stats.fallback.entry("native macro input").or_default() += 1;
            return false;
        };
        let committed = (|| -> Result<(), Abort> {
            for effect in effects {
                match effect {
                    Effect::Define { cs, text } => {
                        let body = result_macro(st, &text)?;
                        st.eqtb.eq_define(
                            &mut st.save_stack,
                            &mut st.mem,
                            &mut st.scanner,
                            cs,
                            CALL,
                            body.0,
                        )?;
                    }
                    Effect::Comparison(value) => {
                        let source = if value { names.if_true } else { names.if_false };
                        let (t, e) = (st.eqtb.eq_type(source), st.eqtb.equiv(source));
                        let flag = names.comparison.expect("comparison");
                        st.eqtb
                            .geq_define(&mut st.mem, &mut st.scanner, flag, t, e)?;
                    }
                }
            }
            Ok(())
        })();
        if committed.is_ok() {
            stats.natives += 1;
        }
        committed.is_ok()
    })
}

/// Print native/fallback counts to stderr when enabled.
pub(crate) fn report() {
    STATE.with(|cell| {
        let state = cell.borrow();
        if state.mode.is_none_or(|m| m == Mode::Off) {
            return;
        }
        let s = &state.stats;
        let fallback: u64 = s.fallback.values().sum();
        eprintln!(
            "pgfmath: {} native, {} plain numbers, {} native macros, {} fallback; checked {}, {} beyond 1e-3, {} format, worst {:.2e}",
            s.native, s.numbers, s.natives, fallback, s.checked, s.differed, s.format_differed, s.worst
        );
        let mut reasons: Vec<_> = s.fallback.iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(a.1));
        for (reason, n) in reasons {
            eprintln!("  {n:>8}  {reason}");
        }
    });
}
