//! Native `\pgfmathparse` in `f64`. The first call of `\pgfmathparse@trynumber@loop`
//! defines `\pgfmathresult` and is redirected to `\pgfmath@parse@end`, which takes
//! the same argument. Results may differ from pgf's macros in the last digits.

use std::collections::HashMap;

use crate::cmd::Cmd;
use crate::codes::ValueLevel;
use crate::consts::HASH_BASE;
use crate::errors::Abort;
use crate::node::{NodePtr, SaElement};
use crate::params::IntPar;
use crate::state::EngineState;
use crate::token::{Token, TokenList};
use crate::units::{EqtbPtr, FontId, MemPtr};

pub(crate) mod eval;

use eval::{Format, Func, Item};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    #[default]
    Off,
    Native,
    Check,
}

impl Mode {
    /// `TEXYON_PGFMATH=native|check|off` overrides `default`.
    pub(crate) fn from_env(default: Mode) -> Mode {
        match std::env::var("TEXYON_PGFMATH").as_deref() {
            Ok("native") => Mode::Native,
            Ok("check") => Mode::Check,
            Ok("off") => Mode::Off,
            _ => default,
        }
    }
}

macro_rules! known_cs {
    (
        required { $($req:ident = $req_name:literal,)* }
        optional { $($opt:ident = $opt_name:literal,)* }
    ) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy)]
        enum Known { $($req,)* $($opt,)* }

        const KNOWN: &[&str] = &[$($req_name,)* $($opt_name,)*];
        const REQUIRED: usize = [$(Known::$req),*].len();

        impl KnownCs {
            $(fn $req(&self) -> EqtbPtr {
                self.cs[Known::$req as usize]
            })*
            $(fn $opt(&self) -> Option<EqtbPtr> {
                Some(self.cs[Known::$opt as usize]).filter(|cs| !cs.is_none())
            })*
        }
    };
}

known_cs! {
    required {
        try_hook = "pgfmathparse@trynumber@loop",
        parse_end = "pgfmath@parse@end",
        is_number = "pgfmathparse@expression@is@number",
        push_operand = "pgfmath@stack@push@operand",
        expression = "pgfmath@expression",
        result = "pgfmathresult",
        units_declared = "ifpgfmathunitsdeclared",
        if_true = "iftrue",
        if_false = "iffalse",
        relax = "relax",
        let_ = "let",
        edef = "edef",
        quickparse = "ifpgfmath@quickparse",
        postparse = "pgfmathpostparse",
        trig_format = "pgfmath@trig@format@choice",
        f_encoding = "f@encoding",
        f_family = "f@family",
        f_series = "f@series",
        f_shape = "f@shape",
        f_size = "f@size",
    }
    optional {
        comparison = "ifpgfmathfloatcomparison",
        fpu_push_operand = "pgfmathfloat@stack@push@operand",
        fpu_is_number = "pgfmathfloat@parse@expression@is@number",
    }
}

/// The control sequences of [`KNOWN`] and [`NATIVES`], zero while a name does
/// not exist. On `EqtbState` and checkpointed with it, so it matches the hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KnownCs {
    cs: [EqtbPtr; KNOWN.len()],
    natives: [EqtbPtr; NATIVES.len()],
}

impl Default for KnownCs {
    fn default() -> Self {
        KnownCs {
            cs: [EqtbPtr::NONE; KNOWN.len()],
            natives: [EqtbPtr::NONE; NATIVES.len()],
        }
    }
}

fn same_name(name: &str, units: &[i32]) -> bool {
    name.len() == units.len() && name.bytes().zip(units).all(|(b, &u)| i32::from(b) == u)
}

impl KnownCs {
    /// Record `cs` if `name`, as UTF-32 units, is a known name.
    pub(crate) fn note(&mut self, name: &[i32], cs: EqtbPtr) {
        for (entry, known) in self.cs.iter_mut().zip(KNOWN) {
            if same_name(known, name) {
                *entry = cs;
            }
        }
        for (entry, (known, _)) in self.natives.iter_mut().zip(NATIVES) {
            if same_name(known, name) {
                *entry = cs;
            }
        }
    }

    /// Look up every name, for a hash that did not go through `note`.
    pub(crate) fn resolve(st: &mut EngineState) {
        for (i, name) in KNOWN.iter().enumerate() {
            st.eqtb.pgfmath.cs[i] = lookup(st, name).unwrap_or(EqtbPtr::NONE);
        }
        for (i, (name, _)) in NATIVES.iter().enumerate() {
            st.eqtb.pgfmath.natives[i] = lookup(st, name).unwrap_or(EqtbPtr::NONE);
        }
    }

    #[inline(always)]
    pub(crate) fn is_try(&self, cs: EqtbPtr) -> bool {
        !cs.is_none() && cs == self.try_hook()
    }

    #[inline(always)]
    pub(crate) fn native(&self, cs: EqtbPtr) -> Option<Native> {
        let i = self.natives.iter().position(|&p| p == cs && !p.is_none())?;
        Some(NATIVES[i].1)
    }

    /// Whether every required name exists. The FPU's names load later.
    fn complete(&self) -> bool {
        self.cs[..REQUIRED].iter().all(|cs| !cs.is_none())
    }

    fn nfss(&self) -> [EqtbPtr; 5] {
        [
            self.f_encoding(),
            self.f_family(),
            self.f_series(),
            self.f_shape(),
            self.f_size(),
        ]
    }
}

/// The macro path would print trace output for these.
const TRACING: &[IntPar] = &[
    IntPar::TracingMacros,
    IntPar::TracingCommands,
    IntPar::TracingAssigns,
    IntPar::TracingGroups,
    IntPar::TracingIfs,
    IntPar::TracingRestores,
    IntPar::TracingOnline,
];

#[derive(Default)]
struct Stats {
    parses: u64,
    numbers: u64,
    macros: u64,
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
    Parsed(eval::Outcome),
}

struct Pending {
    answer: Answer,
    expression: String,
}

/// Counters and check-mode state, for `TEXYON_PGFMATH` runs.
#[derive(Default)]
pub(crate) struct Diagnostics {
    stats: Stats,
    pending: Option<Pending>,
}

/// Returns the control sequence `macro_call` should expand instead.
#[cold]
#[inline(never)]
pub(crate) fn intercept(st: &mut EngineState) -> Option<EqtbPtr> {
    let mode = st.config.pgfmath;
    let known = st.eqtb.pgfmath;
    if !known.complete() {
        return None;
    }
    let body = st.scanner.cur_chr;
    if mode == Mode::Check {
        if let Some(p) = st.pgfmath.pending.take() {
            if body == st.eqtb.equiv(known.parse_end()) || body == st.eqtb.equiv(known.is_number())
            {
                let number = body == st.eqtb.equiv(known.is_number());
                let mut stats = std::mem::take(&mut st.pgfmath.stats);
                compare(st, &known, &mut stats, &p, number);
                st.pgfmath.stats = stats;
                return None;
            }
            st.pgfmath.pending = Some(p);
        }
    }
    if !st.eqtb.pgfmath.is_try(st.scanner.cur_cs) {
        return None;
    }
    let outcome = evaluate_parse(st, &known);
    let stats = &mut st.pgfmath.stats;
    let answer = match outcome {
        Ok(answer) => answer,
        Err(reason) => {
            *stats.fallback.entry(reason).or_default() += 1;
            return None;
        }
    };
    if mode == Mode::Check {
        let expression = expression_tokens(st, known.expression())
            .map(|t| describe(st, &t))
            .unwrap_or_default();
        st.pgfmath.pending = Some(Pending { answer, expression });
        return None;
    }
    match commit(st, &known, &answer) {
        Ok(()) => {
            match answer {
                Answer::Expression => st.pgfmath.stats.numbers += 1,
                Answer::Parsed(_) => st.pgfmath.stats.parses += 1,
            }
            Some(known.parse_end())
        }
        Err(_) => {
            *st.pgfmath
                .stats
                .fallback
                .entry("commit failed")
                .or_default() += 1;
            None
        }
    }
}

fn evaluate_parse(st: &mut EngineState, known: &KnownCs) -> Result<Answer, &'static str> {
    if TRACING.iter().any(|&p| st.eqtb.int_par(p) > 0) || st.eqtb.int_par(IntPar::GlobalDefs) != 0 {
        return Err("tracing or globaldefs");
    }
    if !same_meaning(st, known.quickparse(), known.if_false()) {
        return Err("quick parse");
    }
    let format = match known.fpu_push_operand() {
        Some(fpu) if same_meaning(st, known.push_operand(), fpu) => Format::Float,
        _ => Format::Fixed,
    };
    let float_numbers = known
        .fpu_is_number()
        .is_some_and(|fpu| same_meaning(st, known.is_number(), fpu));
    let tokens = expression_tokens(st, known.expression())?;
    if is_number(&tokens) {
        if !same_meaning(st, known.postparse(), known.relax()) {
            return Err("postparse on a plain number");
        }
        if !float_numbers {
            return Ok(Answer::Expression);
        }
        let text: Option<Vec<u8>> = tokens.iter().map(|t| u8::try_from(t.chr()).ok()).collect();
        return match text.as_deref().and_then(eval::parse_number) {
            Some(v) if v.is_finite() => Ok(Answer::Parsed(eval::Outcome {
                result: eval::float(v),
                units: false,
            })),
            _ => Err("malformed number"),
        };
    }
    let font = selected_font(st, known).ok_or("selectfont would load a font")?;
    let items = expression_items(st, &tokens).inspect_err(|reason| trace(st, &tokens, reason))?;
    let mut constants = HashMap::new();
    for name in identifiers(&items) {
        if Func::from_name(&name).is_none() {
            if let Some(text) = pseudo_constant(st, known, &name) {
                constants.insert(name, text);
            }
        }
    }
    let radians = macro_text(st, known.trig_format()).as_deref() == Some(b"1");
    let constant = |name: &[u8]| constants.get(name).cloned();
    let env = eval::Env {
        font,
        radians,
        format,
        constant: &constant,
    };
    eval::evaluate(&items, &env)
        .map(Answer::Parsed)
        .inspect_err(|reason| trace(st, &tokens, reason))
}

fn lookup(st: &mut EngineState, name: &str) -> Option<EqtbPtr> {
    let buffer: Vec<i32> = name.chars().map(|c| c as i32).collect();
    let saved = st.eqtb.no_new_control_sequence;
    st.eqtb.no_new_control_sequence = true;
    let found = st.eqtb.id_lookup(&mut st.strings, &buffer);
    st.eqtb.no_new_control_sequence = saved;
    match found {
        Ok(p) if p >= HASH_BASE && st.eqtb.text(p).raw() > 0 => Some(p),
        _ => None,
    }
}

fn cs_name(st: &EngineState, cs: EqtbPtr) -> Option<String> {
    if cs < HASH_BASE {
        return None;
    }
    let text = st.eqtb.text(cs);
    if text.raw() <= 0 || text >= st.strings.str_ptr {
        return None;
    }
    crate::introspect::read_string(st, text)
}

fn macro_tokens(st: &EngineState, cs: EqtbPtr) -> Option<Vec<Token>> {
    if !matches!(st.eqtb.eq_type(cs), Cmd::Call | Cmd::LongCall) {
        return None;
    }
    let mut tokens = st.mem.tokens.iter(st.eqtb.equiv_tokens(cs));
    if tokens.next() != Some(Token::END_MATCH) {
        return None;
    }
    Some(tokens.collect())
}

fn macro_text(st: &EngineState, cs: EqtbPtr) -> Option<Vec<u8>> {
    chars(&macro_tokens(st, cs)?)
}

fn same_meaning(st: &EngineState, a: EqtbPtr, b: EqtbPtr) -> bool {
    st.eqtb.eq_type(a) == st.eqtb.eq_type(b) && st.eqtb.equiv(a) == st.eqtb.equiv(b)
}

fn identifiers(items: &[(Item, bool)]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    for &(item, space) in items {
        let c = match item {
            Item::Char { byte: c, .. } => Some(c),
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

fn pseudo_constant(st: &mut EngineState, known: &KnownCs, name: &[u8]) -> Option<Vec<u8>> {
    let name = std::str::from_utf8(name).ok()?;
    let arity = lookup(st, &format!("pgfmath@operation@{name}@arity"))?;
    if macro_text(st, arity).as_deref() != Some(b"0") {
        return None;
    }
    let function = lookup(st, &format!("pgfmath{name}@"))?;
    let body = macro_tokens(st, function)?;
    let is = |t: &Token, cs: EqtbPtr| t.is_cs() && same_meaning(st, t.cs(), cs);
    let result = |t: &Token| t.is_cs() && t.cs() == known.result();
    let char_is = |t: &Token, c: u8| !t.is_cs() && t.chr() == i32::from(c);
    let target = match body.as_slice() {
        [l, r, eq, t] if is(l, known.let_()) && result(r) && char_is(eq, b'=') && t.is_cs() => {
            t.cs()
        }
        [l, r, t] if is(l, known.let_()) && result(r) && t.is_cs() => t.cs(),
        [d, r, open, t, close]
            if is(d, known.edef())
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

fn expanded_text(st: &EngineState, mut cs: EqtbPtr) -> Option<Vec<u8>> {
    for _ in 0..8 {
        let tokens = macro_tokens(st, cs)?;
        match tokens.as_slice() {
            [t] if t.is_cs() => cs = t.cs(),
            _ => return macro_text(st, cs),
        }
    }
    None
}

fn expression_tokens(st: &EngineState, expression: EqtbPtr) -> Result<Vec<Token>, &'static str> {
    macro_tokens(st, expression).ok_or("expression is not a plain macro")
}

fn is_number(tokens: &[Token]) -> bool {
    tokens.iter().all(|t| {
        !t.is_cs()
            && !matches!(t.cmd(), Cmd::LeftBrace | Cmd::RightBrace | Cmd::Spacer)
            && (t.chr() == i32::from(b'.')
                || (i32::from(b'0')..=i32::from(b'9')).contains(&t.chr()))
    })
}

fn expression_items(st: &EngineState, tokens: &[Token]) -> Result<Vec<(Item, bool)>, &'static str> {
    let mut body = tokens;
    // pgf strips a lone outer brace group.
    while body
        .first()
        .is_some_and(|t| !t.is_cs() && t.cmd() == Cmd::LeftBrace)
    {
        let mut depth = 0;
        let mut close = None;
        for (i, t) in body.iter().enumerate() {
            if t.is_cs() {
                continue;
            }
            match t.cmd() {
                Cmd::LeftBrace => depth += 1,
                Cmd::RightBrace => {
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
            Cmd::Spacer => space = true,
            Cmd::Letter
            | Cmd::OtherChar
            | Cmd::SupMark
            | Cmd::SubMark
            | Cmd::TabMark
            | Cmd::MathShift
                if chr < 128 =>
            {
                items.push((
                    Item::Char {
                        byte: chr as u8,
                        other: cmd == Cmd::OtherChar,
                    },
                    space,
                ));
                space = false;
            }
            _ => return Err("unsupported token"),
        }
    }
    Ok(items)
}

fn register_item(st: &EngineState, cs: EqtbPtr) -> Result<Item, &'static str> {
    if cs < HASH_BASE {
        return Err("active or single-character control sequence");
    }
    let m = st.eqtb.equiv(cs);
    match st.eqtb.eq_type(cs) {
        Cmd::AssignDimen => Ok(Item::Dimen(st.eqtb.int_at(st.eqtb.equiv_loc(cs)))),
        Cmd::AssignInt => Ok(Item::Count(st.eqtb.int_at(st.eqtb.equiv_loc(cs)))),
        Cmd::Register if ValueLevel::from_raw(m).is_none() => {
            let sa = NodePtr::<SaElement>::at(MemPtr::from_raw(m));
            match (sa.word(&st.mem), st.mem[sa].sa_type().get()) {
                (Some(w), Some(ValueLevel::Int)) => Ok(Item::Count(st.mem[w].sa_int)),
                (Some(w), Some(ValueLevel::Dimen)) => Ok(Item::Dimen(st.mem[w].sa_int)),
                _ => Err("unsupported register"),
            }
        }
        _ => Err("unsupported control sequence"),
    }
}

/// `None` when `\selectfont` would load the font, which only the macros can do.
fn selected_font(st: &mut EngineState, known: &KnownCs) -> Option<eval::Font> {
    let mut name = String::new();
    for (i, &cs) in known.nfss().iter().enumerate() {
        if i > 0 {
            name.push('/');
        }
        name.push_str(std::str::from_utf8(&macro_text(st, cs)?).ok()?);
    }
    let id = lookup(st, &name)?;
    if st.eqtb.eq_type(id) != Cmd::SetFont {
        return None;
    }
    let font = FontId::try_from(st.eqtb.equiv(id)).ok()?;
    Some(eval::Font {
        quad: st.fonts.param(font, crate::fonts::QUAD_CODE).raw(),
        x_height: st.fonts.param(font, crate::fonts::X_HEIGHT_CODE).raw(),
    })
}

/// Letters get catcode 11 like the `Y` and `e` of `\pgfmathfloatcreate`.
fn result_macro(st: &mut EngineState, text: &[u8]) -> Result<TokenList, Abort> {
    let out = st.mem.tokens.begin()?;
    st.mem.tokens.push(&out, Token::END_MATCH)?;
    for &b in text {
        let tok = if b.is_ascii_alphabetic() {
            Token::letter(b)
        } else {
            Token::other(b)
        };
        st.mem.tokens.push(&out, tok)?;
    }
    st.mem.tokens.finish(out)
}

fn commit(st: &mut EngineState, known: &KnownCs, answer: &Answer) -> Result<(), Abort> {
    let body = match answer {
        Answer::Expression => {
            let shared = st.eqtb.equiv_tokens(known.expression());
            st.mem.tokens.add_ref(shared);
            shared
        }
        Answer::Parsed(outcome) => {
            if outcome.units {
                let (t, e) = (
                    st.eqtb.eq_type(known.if_true()),
                    st.eqtb.equiv(known.if_true()),
                );
                st.eqtb
                    .geq_define(&mut st.mem, &mut st.scanner, known.units_declared(), t, e)?;
            }
            result_macro(st, &outcome.result)?
        }
    };
    st.eqtb.eq_define(
        &mut st.save_stack,
        &mut st.mem,
        &mut st.scanner,
        known.result(),
        Cmd::Call,
        body.raw(),
    )
}

fn compare(st: &EngineState, known: &KnownCs, stats: &mut Stats, pending: &Pending, number: bool) {
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
    let Some(theirs) = macro_text(st, known.result()) else {
        return;
    };
    let shape = |t: &[u8]| (t.contains(&b'Y'), t.contains(&b'.'));
    if shape(&theirs) != shape(ours)
        || units != same_meaning(st, known.units_declared(), known.if_true())
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
            text.push_str(
                &cs_name(st, tok.cs()).unwrap_or_else(|| format!("<{}>", tok.cs().raw())),
            );
            text.push(' ');
        } else {
            text.push(char::from_u32(tok.chr() as u32).unwrap_or('?'));
        }
    }
    text
}

fn trace(st: &EngineState, tokens: &[Token], reason: &str) {
    let Ok(filter) = std::env::var("TEXYON_PGFMATH_TRACE") else {
        return;
    };
    if reason.contains(filter.as_str()) {
        eprintln!("pgfmath fallback ({reason}): {}", describe(st, tokens));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Native {
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

enum Effect {
    Define { cs: EqtbPtr, text: Vec<u8> },
    Comparison(bool),
}

fn chars(tokens: &[Token]) -> Option<Vec<u8>> {
    tokens
        .iter()
        .map(|t| {
            (!t.is_cs() && t.chr() < 128 && !matches!(t.cmd(), Cmd::LeftBrace | Cmd::RightBrace))
                .then_some(t.chr() as u8)
        })
        .collect()
}

fn expand_full(st: &EngineState, tokens: &[Token], depth: u32) -> Option<Vec<u8>> {
    if depth > 16 {
        return None;
    }
    let mut out = Vec::new();
    for t in tokens {
        if t.is_cs() {
            out.extend(expand_full(st, &macro_tokens(st, t.cs())?, depth + 1)?);
        } else {
            out.extend(chars(std::slice::from_ref(t))?);
        }
    }
    Some(out)
}

fn expand_once(st: &EngineState, tokens: &[Token]) -> Option<Vec<u8>> {
    match tokens.split_first() {
        Some((first, rest)) if first.is_cs() => {
            let mut all = macro_tokens(st, first.cs())?;
            all.extend_from_slice(rest);
            chars(&all)
        }
        _ => chars(tokens),
    }
}

fn split_float(text: &[u8]) -> Option<(u8, &[u8], i32)> {
    if text.len() < 5 || text[1] != b'Y' || *text.last()? != b']' || !text[0].is_ascii_digit() {
        return None;
    }
    let body = &text[2..text.len() - 1];
    let e = body.iter().rposition(|&c| c == b'e')?;
    let exponent = std::str::from_utf8(&body[e + 1..]).ok()?.parse().ok()?;
    Some((text[0], &body[..e], exponent))
}

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
    known: &KnownCs,
    kind: Native,
    args: &[Vec<Token>],
) -> Option<Vec<Effect>> {
    let result = |text: Vec<u8>| Effect::Define {
        cs: known.result(),
        text,
    };
    let less_than = |a: &[Token], b: &[Token]| -> Option<(bool, Vec<u8>, Vec<u8>)> {
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
            let (less, _, _) = less_than(a, b)?;
            known.comparison()?;
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
            let (less, ta, tb) = less_than(a, b)?;
            known.comparison()?;
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

/// True when answered, then `macro_call` frees the arguments and skips the body.
#[cold]
#[inline(never)]
pub(crate) fn native_macro(st: &mut EngineState, kind: Native, args: &[TokenList]) -> bool {
    if TRACING.iter().any(|&p| st.eqtb.int_par(p) > 0) || st.eqtb.int_par(IntPar::GlobalDefs) != 0 {
        return false;
    }
    let known = st.eqtb.pgfmath;
    if !known.complete() {
        return false;
    }
    let args: Vec<Vec<Token>> = args
        .iter()
        .map(|&p| st.mem.tokens.iter(p).collect())
        .collect();
    let Some(effects) = run_native(st, &known, kind, &args) else {
        *st.pgfmath
            .stats
            .fallback
            .entry("native macro input")
            .or_default() += 1;
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
                        Cmd::Call,
                        body.raw(),
                    )?;
                }
                Effect::Comparison(value) => {
                    let source = if value {
                        known.if_true()
                    } else {
                        known.if_false()
                    };
                    let (t, e) = (st.eqtb.eq_type(source), st.eqtb.equiv(source));
                    let flag = known.comparison().expect("comparison");
                    st.eqtb
                        .geq_define(&mut st.mem, &mut st.scanner, flag, t, e)?;
                }
            }
        }
        Ok(())
    })();
    if committed.is_ok() {
        st.pgfmath.stats.macros += 1;
    }
    committed.is_ok()
}

pub(crate) fn report(st: &EngineState) {
    if std::env::var_os("TEXYON_PGFMATH").is_none() {
        return;
    }
    let s = &st.pgfmath.stats;
    let fallback: u64 = s.fallback.values().sum();
    eprintln!(
        "pgfmath: {} native, {} plain numbers, {} native macros, {} fallback; checked {}, {} beyond 1e-3, {} format, worst {:.2e}",
        s.parses, s.numbers, s.macros, fallback, s.checked, s.differed, s.format_differed, s.worst
    );
    let mut reasons: Vec<_> = s.fallback.iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(a.1));
    for (reason, n) in reasons {
        eprintln!("  {n:>8}  {reason}");
    }
}
