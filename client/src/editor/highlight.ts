/**
 * What the tokens look like.
 *
 * CodeMirror 6 separates parsing from painting: the language mode assigns
 * highlight *tags* and nothing renders them until a style says how. Without
 * this the `stex` mode still tokenizes the document on every keystroke and
 * produces no visible difference at all, which is a quiet way to be broken.
 *
 * Written out rather than taking `defaultHighlightStyle`, which is tuned for a
 * light background — this editor is dark, and the point of colouring a document
 * is that the colours can be told apart.
 */

import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { tags } from "@lezer/highlight";

/**
 * A palette for TeX, arranged by what a reader is looking for.
 *
 * The three that carry the document are commands, the identifiers inside their
 * braces, and everything else. Braces themselves are dimmed rather than
 * coloured: there are a great many of them and they are punctuation. Prose is
 * left the plain text colour, because it is what the document is *for* and
 * colouring it would be colouring everything.
 *
 * The tags below are what the `stex` stream mode emits, mapped through the
 * bridge in @codemirror/language; the comments say which construct each one
 * turned out to cover in practice rather than what its name suggests.
 */
const style = HighlightStyle.define([
  // Every control sequence — `\section`, `\label`, `\vect` — and the `$` that
  // opens and closes maths.
  { tag: tags.tagName, color: "#7aa2f7" },
  { tag: tags.keyword, color: "#7aa2f7" },
  // The identifiers inside significant braces: `sec:method` in a `\label`,
  // `equation` in a `\begin`, `article` in a `\documentclass`. These are the
  // names a reader scans for, so they are the second-strongest colour.
  { tag: tags.atom, color: "#bb9af7" },
  { tag: tags.string, color: "#9ece6a" },
  // The mode calls these "builtin"; the bridge maps that to
  // variableName.standard.
  { tag: tags.standard(tags.variableName), color: "#e0af68" },
  { tag: tags.number, color: "#ff9e64" },
  // Braces and brackets: present, but not competing with the content.
  { tag: tags.bracket, color: "#5c6370" },
  { tag: tags.comment, color: "#5c6370", fontStyle: "italic" },
  // What the mode could not make sense of — usually an unbalanced brace, which
  // is worth seeing before the compiler says so.
  { tag: tags.invalid, color: "#ff6b6b", textDecoration: "underline wavy" },
]);

/** Install both the style and the extension that actually paints with it. */
export const latexHighlighting: Extension = syntaxHighlighting(style);
