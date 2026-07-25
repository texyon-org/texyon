/**
 * What to offer when someone is halfway through typing.
 *
 * Two sources, and the project's own is the one that earns its place. A list of
 * LaTeX's built-in commands is mildly useful; the label you wrote two files ago
 * is what you actually cannot remember.
 */

import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";

import type { SymbolIndex } from "../project/symbols";

/**
 * Commands worth offering that no project declares.
 *
 * Deliberately short. This is not a LaTeX reference — offering three thousand
 * commands buries the handful that matter under an alphabet — it is the set
 * someone types often enough to want completed.
 */
const BUILTIN: readonly { name: string; detail: string }[] = [
  { name: "section", detail: "sectioning" },
  { name: "subsection", detail: "sectioning" },
  { name: "subsubsection", detail: "sectioning" },
  { name: "paragraph", detail: "sectioning" },
  { name: "chapter", detail: "sectioning" },
  { name: "textbf", detail: "bold" },
  { name: "textit", detail: "italic" },
  { name: "texttt", detail: "monospace" },
  { name: "emph", detail: "emphasis" },
  { name: "footnote", detail: "note" },
  { name: "begin", detail: "environment" },
  { name: "end", detail: "environment" },
  { name: "item", detail: "list" },
  { name: "caption", detail: "float" },
  { name: "includegraphics", detail: "graphics" },
  { name: "label", detail: "cross-reference" },
  { name: "ref", detail: "cross-reference" },
  { name: "eqref", detail: "cross-reference" },
  { name: "pageref", detail: "cross-reference" },
  { name: "cite", detail: "bibliography" },
  { name: "usepackage", detail: "preamble" },
  { name: "documentclass", detail: "preamble" },
  { name: "input", detail: "include" },
  { name: "include", detail: "include" },
  { name: "frac", detail: "maths" },
  { name: "sqrt", detail: "maths" },
  { name: "sum", detail: "maths" },
  { name: "int", detail: "maths" },
];

/** Environments worth offering inside `\begin{…}`. */
const BUILTIN_ENVIRONMENTS: readonly string[] = [
  "document",
  "itemize",
  "enumerate",
  "description",
  "figure",
  "table",
  "tabular",
  "equation",
  "align",
  "gather",
  "quote",
  "verbatim",
  "abstract",
];

/**
 * Completions for LaTeX, aware of what this project declares.
 *
 * `symbols` is read through a function rather than captured, because the index
 * is rebuilt as the project changes and a completion source installed once must
 * see the current one.
 */
export function latexCompletions(symbols: () => SymbolIndex) {
  return (context: CompletionContext): CompletionResult | null => {
    // `\ref{sec:` and friends: what belongs here is a label, and nothing else.
    const reference = context.matchBefore(/\\(?:eq|page|auto|c|name)?ref\s*\{[^}]*/);
    if (reference !== null) {
      return {
        from: reference.from + reference.text.indexOf("{") + 1,
        options: symbols().labels.map((label) => ({
          label: label.name,
          type: "constant",
          detail: `${label.path}:${label.line}`,
        })),
        validFor: /^[^}]*$/,
      };
    }

    const citation = context.matchBefore(/\\(?:no|foot|text|paren|auto)?cite[a-zA-Z]*\s*\{[^}]*/);
    if (citation !== null) {
      return {
        from: citation.from + citation.text.indexOf("{") + 1,
        options: symbols().citations.map((entry) => ({
          label: entry.name,
          type: "constant",
          detail: entry.path,
        })),
        validFor: /^[^}]*$/,
      };
    }

    const environment = context.matchBefore(/\\(?:begin|end)\s*\{[^}]*/);
    if (environment !== null) {
      const declared = symbols().environments.map(
        (found): Completion => ({
          label: found.name,
          type: "class",
          detail: found.path,
        }),
      );
      const builtin = BUILTIN_ENVIRONMENTS.map(
        (name): Completion => ({ label: name, type: "class" }),
      );

      return {
        from: environment.from + environment.text.indexOf("{") + 1,
        options: [...declared, ...builtin],
        validFor: /^[^}]*$/,
      };
    }

    // A bare command. Only when the user has typed the backslash: offering
    // every command against an empty prefix is noise.
    const command = context.matchBefore(/\\[a-zA-Z@]*/);
    if (command === null || (command.from === command.to && !context.explicit)) {
      return null;
    }

    const declared = symbols().commands.map(
      (found): Completion => ({
        label: found.name,
        type: "function",
        detail:
          found.arity === undefined
            ? found.path
            : `${found.path} · ${found.arity} arg${found.arity === 1 ? "" : "s"}`,
        // Declared in this project, so it outranks anything built in.
        boost: 1,
      }),
    );

    return {
      // After the backslash, which stays.
      from: command.from + 1,
      options: [
        ...declared,
        ...BUILTIN.map((entry): Completion => ({
          label: entry.name,
          type: "keyword",
          detail: entry.detail,
        })),
      ],
      validFor: /^[a-zA-Z@]*$/,
    };
  };
}
