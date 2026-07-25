//! What the engine currently has defined.
//!
//! An editor offering completions for LaTeX has to know which commands exist,
//! and every way of finding that out from outside is an approximation: reading
//! a package's `.sty` misses anything built in a loop or with `\csname`, and a
//! bundled database goes stale and never knew about the document's own
//! preamble. After the preamble has run, TeX's hash table already holds the
//! exact answer — this reads it.
//!
//! Nothing here changes engine state. It is a read-only walk, safe to run
//! between operations on a resident session.

use crate::consts::{EQTB_SIZE, FROZEN_CONTROL_SEQUENCE, TOO_BIG_CHAR};
use crate::state::EngineState;

/// First slot holding a multi-letter control sequence.
const HASH_BASE: i32 = 2_228_226;

/// Command code of the first macro-like meaning. Anything at or above this is
/// a macro with a body; below it is a primitive, a register, a font, and so on.
const CALL: u16 = 113;

/// The meaning a control sequence has when it has none.
const UNDEFINED_CS: u16 = 103;

/// A `#1` in a macro's parameter text. Tokens from here up to
/// [`END_MATCH_TOKEN`] are the arguments the macro takes.
const MATCH_TOKEN: i32 = 0x1A0_0000;

/// The end of a macro's parameter text; its body follows.
const END_MATCH_TOKEN: i32 = 0x1C0_0000;

/// `\protected` marks itself with this, as the first token of the body.
const PROTECTED_TOKEN: i32 = END_MATCH_TOKEN + 1;

/// What kind of thing a name is bound to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKind {
    /// A macro with a body — anything `\newcommand`, `\def` or a package made.
    Macro,
    /// A macro that will not expand while TeX is looking ahead.
    ProtectedMacro,
    /// A primitive, a register, a font, a character class: defined, but not a
    /// macro, so it has no parameter text and no arity to report.
    Builtin,
}

/// One control sequence the engine has defined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinedCommand {
    /// The name, without its leading backslash.
    pub name: String,
    /// How many arguments it takes, including an optional one if it has it.
    pub arity: u8,
    /// Whether the first argument is optional — `\foo[x]{y}` rather than
    /// `\foo{x}{y}`.
    pub optional: bool,
    /// Whether it is a macro, and whether it is protected.
    pub kind: CommandKind,
}

/// Every control sequence the engine currently has defined.
///
/// Includes internals — names full of `@` and `_` that no one types — because
/// what counts as internal is a matter of taste and belongs to whoever is
/// presenting the list rather than to the engine. What it does not include is
/// the companion names LaTeX hides real definitions in; see
/// [`resolve_wrappers`].
pub(crate) fn defined_commands(st: &EngineState) -> Vec<DefinedCommand> {
    let mut found = Vec::new();

    // Multi-letter control sequences occupy one contiguous run of the table.
    for slot in HASH_BASE..FROZEN_CONTROL_SEQUENCE {
        collect(st, slot, &mut found);
    }

    // A document that defines more names than the table was sized for spills
    // into a second region above the end of eqtb.
    let high = st.eqtb.hash_high;
    for offset in 1..=high {
        collect(st, EQTB_SIZE + offset, &mut found);
    }

    resolve_wrappers(found)
}

/// Recover the arity that LaTeX's robust-command machinery hides.
///
/// This is the one place here that knows about LaTeX rather than about TeX, and
/// it earns its keep: without it `\frac` reports no arguments, because what
/// `\frac` actually contains is a instruction to expand something else.
///
/// LaTeX has two conventions for that, and both put the real definition in a
/// name that cannot be typed:
///
/// - `\DeclareRobustCommand{\frac}[2]` defines `\frac` as a jump and the body
///   as `frac␣` — the name with a trailing space.
/// - `\newcommand{\foo}[2][d]` defines `\foo` as optional-argument machinery
///   and the body as `\foo` — the name with a leading backslash. A command
///   wrapped this way has an optional first argument by construction, which is
///   how that is recovered.
///
/// Commands built on `\@ifstar`, `\@ifnextchar` or `\@startsection` — which
/// includes `\section` and `\includegraphics` — are not recoverable at all:
/// their arguments are decided by macros that inspect the input as it is read,
/// so there is no parameter text to count anywhere. They report zero, honestly.
fn resolve_wrappers(found: Vec<DefinedCommand>) -> Vec<DefinedCommand> {
    let arity_of: std::collections::HashMap<&str, u8> = found
        .iter()
        .map(|command| (command.name.as_str(), command.arity))
        .collect();

    let mut resolved = Vec::with_capacity(found.len());

    for command in &found {
        // The companions themselves are artifacts, not things anyone types.
        if command.name.ends_with(' ') || command.name.starts_with('\\') {
            continue;
        }

        let mut entry = command.clone();

        if entry.arity == 0 && entry.kind != CommandKind::Builtin {
            if let Some(&arity) = arity_of.get(format!("{} ", entry.name).as_str()) {
                entry.arity = arity;
            } else if let Some(&arity) = arity_of.get(format!("\\{}", entry.name).as_str()) {
                entry.arity = arity;
                entry.optional = true;
            }
        }

        resolved.push(entry);
    }

    resolved
}

/// Add the control sequence at one table slot, if it holds one.
fn collect(st: &EngineState, slot: i32, found: &mut Vec<DefinedCommand>) {
    let text = st.eqtb.hash_s1(slot);

    // An empty slot, or one naming a string that has since been discarded.
    if text <= 0 || text >= st.strings.str_ptr {
        return;
    }

    let meaning = st.eqtb.b16s1(slot);
    if meaning == UNDEFINED_CS {
        return;
    }

    let Some(name) = read_string(st, text) else {
        return;
    };

    let (kind, arity) = if meaning >= CALL {
        let body = st.eqtb.b32s1(slot);
        let (protected, arity) = describe_macro(st, body);
        (
            if protected {
                CommandKind::ProtectedMacro
            } else {
                CommandKind::Macro
            },
            arity,
        )
    } else {
        (CommandKind::Builtin, 0)
    };

    found.push(DefinedCommand {
        name,
        arity,
        optional: false,
        kind,
    });
}

/// Count a macro's arguments, and note whether it is `\protected`.
///
/// A macro's token list opens with a reference count, then its parameter text,
/// then [`END_MATCH_TOKEN`], then the body. Every `#n` in the parameter text is
/// one argument, which is how TeX itself knows how many to collect.
fn describe_macro(st: &EngineState, body: i32) -> (bool, u8) {
    let mut protected = false;
    let mut arity = 0u8;

    // Past the reference count, which is bookkeeping rather than a token.
    let mut at = st.mem.b32s1(body);

    // A macro can in principle take nine arguments; the bound is a guard
    // against following a corrupt list rather than a real limit.
    for _ in 0..64 {
        if at <= 0 {
            break;
        }

        let token = st.mem.b32s0(at);

        if token == PROTECTED_TOKEN {
            protected = true;
        } else if token >= END_MATCH_TOKEN {
            break;
        } else if token >= MATCH_TOKEN {
            arity = arity.saturating_add(1);
        }

        at = st.mem.b32s1(at);
    }

    (protected, arity)
}

/// Recover one string from the pool.
///
/// The pool holds UTF-16 code units, so a name outside the basic plane arrives
/// as a surrogate pair and has to be reassembled rather than copied.
fn read_string(st: &EngineState, index: i32) -> Option<String> {
    let start = *st.strings.str_start.get((index - TOO_BIG_CHAR) as usize)? as usize;
    let end = *st
        .strings
        .str_start
        .get((index + 1 - TOO_BIG_CHAR) as usize)? as usize;

    if end < start || end > st.strings.str_pool.len() {
        return None;
    }

    let units = &st.strings.str_pool[start..end];
    char::decode_utf16(units.iter().copied())
        .collect::<Result<String, _>>()
        .ok()
}
