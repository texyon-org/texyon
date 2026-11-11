//! Read-only listing of the control sequences the engine has defined, for
//! editor completions. Safe to run between operations on a resident session.

use crate::token::Token;
use crate::units::{EqtbPtr, StrNumber};
use std::collections::HashMap;

use crate::cmd::Cmd;
use crate::consts::{EQTB_SIZE, FROZEN_CONTROL_SEQUENCE, HASH_BASE};
use crate::state::EngineState;

/// What kind of thing a name is bound to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKind {
    /// A macro with a body.
    Macro,
    /// A `\protected` macro.
    ProtectedMacro,
    /// Defined but not a macro, so it has no arity to report.
    Builtin,
}

/// One control sequence the engine has defined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinedCommand {
    /// The name, without its leading backslash.
    pub name: String,
    /// Argument count, including an optional first argument.
    pub arity: u8,
    /// Whether the first argument is optional.
    pub optional: bool,
    /// Whether it is a macro, and whether it is protected.
    pub kind: CommandKind,
}

/// Every defined control sequence, internals included, with LaTeX's hidden
/// companion names folded into their public command (see [`resolve_wrappers`]).
pub(crate) fn defined_commands(st: &EngineState) -> Vec<DefinedCommand> {
    let mut found = Vec::new();
    for slot in HASH_BASE.until(FROZEN_CONTROL_SEQUENCE) {
        collect(st, slot, &mut found);
    }
    for offset in 1..=st.eqtb.hash_high {
        collect(st, EQTB_SIZE + offset, &mut found);
    }
    resolve_wrappers(found)
}

/// Recovers the arity LaTeX hides in an untypeable companion name.
///
/// `\DeclareRobustCommand{\frac}` puts the body in `frac␣` (trailing space) and
/// `\newcommand{\foo}` with an optional argument puts it in `\foo` (leading
/// backslash in the name). Commands built on `\@ifstar` and friends have no
/// parameter text anywhere and report zero.
fn resolve_wrappers(found: Vec<DefinedCommand>) -> Vec<DefinedCommand> {
    let arity_of: HashMap<&str, u8> = found
        .iter()
        .map(|command| (command.name.as_str(), command.arity))
        .collect();

    let mut resolved = Vec::with_capacity(found.len());
    for command in &found {
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

fn collect(st: &EngineState, slot: EqtbPtr, found: &mut Vec<DefinedCommand>) {
    let text = st.eqtb.text(slot);
    if text <= StrNumber::NONE || text >= st.strings.str_ptr {
        return;
    }
    let meaning = st.eqtb.eq_type(slot);
    if meaning == Cmd::UndefinedCs {
        return;
    }
    let Some(name) = read_string(st, text) else {
        return;
    };

    let (kind, arity) = if meaning >= Cmd::Call {
        let (protected, arity) = describe_macro(st, st.eqtb.equiv_tokens(slot));
        let kind = if protected {
            CommandKind::ProtectedMacro
        } else {
            CommandKind::Macro
        };
        (kind, arity)
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

/// Returns whether the macro is `\protected` and how many `#n` its parameter text has.
fn describe_macro(st: &EngineState, body: crate::token::TokenList) -> (bool, u8) {
    let mut protected = false;
    let mut arity = 0u8;
    if !st.mem.tokens.holds_list(body) {
        return (protected, arity);
    }
    let mut at = st.mem.tokens.cursor(body);

    // Guards against following a corrupt list.
    for _ in 0..64 {
        if !st.mem.tokens.holds(at) {
            break;
        }
        let Some(token) = st.mem.tokens.read(&mut at) else {
            break;
        };
        if token == Token::PROTECTED {
            protected = true;
        } else if token >= Token::END_MATCH {
            break;
        } else if token.is_match() {
            arity = arity.saturating_add(1);
        }
    }

    (protected, arity)
}

pub(crate) fn read_string(st: &EngineState, s: StrNumber) -> Option<String> {
    let str_start = &st.strings.str_start;
    let start = str_start.get(s.slot())?.index();
    let end = str_start.get(s.next().slot())?.index();
    let units = st.strings.str_pool.get(start..end)?;
    char::decode_utf16(units.iter().copied())
        .collect::<Result<String, _>>()
        .ok()
}
