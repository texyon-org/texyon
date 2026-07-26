/**
 * Turning what we know about a command into something you can type into.
 *
 * Accepting `\vect` should leave the cursor inside its argument, not after its
 * name — and accepting an environment should write the `\end` too, with both
 * halves editable together so renaming one renames the other. CodeMirror does
 * all of that through snippet templates; this decides what template each
 * command deserves.
 *
 * Template syntax, from `@codemirror/autocomplete`: `${}` is a field to tab
 * through, `${name}` is a field pre-filled with that name, and `${0}` is where
 * the cursor finishes. Every instance of the same field is selected together,
 * which is what makes `\begin{x}…\end{x}` rename as one.
 */

import type { CwlArgument } from "../project/cwl";
import type { MergedCommand } from "../project/commands";

/** A `{` or `}` that is part of the text rather than a field. */
function literal(text: string): string {
  return text.replace(/[{}]/g, (brace) => `\\${brace}`);
}

/**
 * The template for a command, without its leading backslash.
 *
 * Returns null for a command that takes nothing, which is most of them — a
 * plain insertion is better than a snippet with no fields, because it leaves no
 * active snippet session to escape from.
 */
export function commandSnippet(command: MergedCommand): string | null {
  const signature = command.signatures?.[0];

  if (signature !== undefined && signature.length > 0) {
    return command.name + writeArguments(signature);
  }

  if (command.arity === 0) return null;

  // No names to offer, so empty fields. The first argument is bracketed when
  // the engine reported the command as taking an optional one.
  let template = command.name;
  for (let index = 0; index < command.arity; index++) {
    template += index === 0 && command.optional ? "[${}]" : "\\{${}\\}";
  }
  return template;
}

/** Write one signature's arguments as fields. */
function writeArguments(signature: readonly CwlArgument[]): string {
  let template = "";
  for (const argument of signature) {
    // A field pre-filled with the argument's own name, so `\frac` arrives as
    // `\frac{num}{den}` with `num` selected and ready to be typed over.
    const field = `\${${literalField(argument.name)}}`;
    template += argument.optional ? `[${field}]` : `\\{${field}\\}`;
  }
  return template;
}

/**
 * A field's default text.
 *
 * Braces would end the field early, and an empty name gives an empty field,
 * which is the right thing when the corpus did not name the argument.
 */
function literalField(name: string): string {
  return name.replace(/[{}]/g, "").trim();
}

/**
 * The template for opening an environment.
 *
 * Written for the moment someone is completing inside `\begin{`, so it leaves a
 * line for the body and writes the matching `\end`. The environment name
 * appears as the same field twice, which is what makes editing one edit the
 * other.
 *
 * It always closes its own brace. Typing `\begin{` makes the editor's bracket
 * closing supply one too, but that one sits *after* the whole insertion — the
 * newline and the `\end` go in front of it — so omitting ours would give
 * `\begin{itemize` open and `\end{itemize}}` closed twice. The stray brace is
 * swallowed by the completion instead; see `consumingBrace`.
 *
 * Optional arguments are left out. A field pre-filled with the argument's own
 * name is right for a mandatory one — `\frac{num}{den}` has to be filled in
 * either way — but an optional one that is never touched stays in the
 * document, and `\begin{tikzpicture}[options]` does not compile: tikz reads
 * `options` as a key it does not have. Someone who wants them types `[`.
 */
export function environmentSnippet(
  name: string,
  args: readonly CwlArgument[] = [],
): string {
  const field = `\${${literal(name)}}`;
  const required = args.filter((argument) => !argument.optional);
  const opening = required.length > 0 ? writeArguments(required) : "";

  // The tab indents the body by one unit; the newline before \end puts the
  // closing tag back at the outer level.
  return `${field}\\}${opening}\n\t\${0}\n\\end\\{${field}\\}`;
}

/**
 * A short description of how a command is written, for the completion list.
 *
 * `\frac{num}{den}` reads better beside a name than "2 arguments" does.
 */
export function describeSignature(command: MergedCommand): string {
  const signature = command.signatures?.[0];

  if (signature !== undefined && signature.length > 0) {
    return signature
      .map((argument) => (argument.optional ? `[${argument.name}]` : `{${argument.name}}`))
      .join("");
  }

  if (command.arity === 0) return "";

  return command.optional
    ? "[]" + "{}".repeat(Math.max(0, command.arity - 1))
    : "{}".repeat(command.arity);
}
