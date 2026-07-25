/**
 * What to offer when someone is halfway through typing.
 *
 * Three things know something, and each answers a different question. The
 * project's own files know its labels, citation keys and `\newcommand`s. The
 * engine knows every command this document actually has, and how many arguments
 * each takes. The corpus knows what those arguments are called, where a command
 * is legal, and — for a command the document does *not* have — which package
 * would provide it.
 *
 * Everything offered here is a snippet rather than a word, so accepting a
 * completion leaves the cursor in the first argument rather than after the
 * name.
 */

import { snippetCompletion, type Completion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";

import { availableCommand, type MergedCommand } from "../project/commands";
import type { SymbolIndex } from "../project/symbols";
import { commandSnippet, describeSignature, environmentSnippet } from "./snippets";

/** Everything the completion source reads, fetched fresh on each request. */
export interface CompletionSources {
  symbols: SymbolIndex;
  /** The merged command set: everything this document actually has. */
  commands: readonly MergedCommand[];
  /** Every command the corpus knows of, mapped to a package providing it. */
  providers: ReadonlyMap<string, string>;
  /** Add `\usepackage{name}` to the preamble. */
  addPackage: (name: string) => void;
}

/**
 * Environments worth offering before a document has been compiled.
 *
 * Once it has, the corpus supplies these and more; this is only so a fresh
 * project is not inert.
 */
const FALLBACK_ENVIRONMENTS = [
  "document", "itemize", "enumerate", "description", "figure", "table",
  "tabular", "equation", "align", "gather", "quote", "verbatim", "abstract",
] as const;

/** Completions for LaTeX, aware of what this document actually has. */
export function latexCompletions(sources: () => CompletionSources) {
  return (context: CompletionContext): CompletionResult | null => {
    const at = sources();

    // `\ref{sec:` and friends: what belongs here is a label, and nothing else.
    const reference = context.matchBefore(/\\(?:eq|page|auto|c|name)?ref\s*\{[^}]*/);
    if (reference !== null) {
      return {
        from: reference.from + reference.text.indexOf("{") + 1,
        options: at.symbols.labels.map((label) => ({
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
        options: at.symbols.citations.map((entry) => ({
          label: entry.name,
          type: "constant",
          detail: entry.path,
        })),
        validFor: /^[^}]*$/,
      };
    }

    // Inside `\begin{`: choosing an environment also writes its `\end`.
    const begin = context.matchBefore(/\\begin\s*\{[^}]*/);
    if (begin !== null) {
      return {
        from: begin.from + begin.text.indexOf("{") + 1,
        options: environmentOptions(at),
        validFor: /^[^}]*$/,
      };
    }

    // `\end{` closes whatever is open, so it needs no snippet.
    const end = context.matchBefore(/\\end\s*\{[^}]*/);
    if (end !== null) {
      return {
        from: end.from + end.text.indexOf("{") + 1,
        options: environmentNames(at).map((name) => ({ label: name, type: "class" })),
        validFor: /^[^}]*$/,
      };
    }

    // A bare command. Only once the backslash is typed: offering every command
    // against an empty prefix is noise.
    const command = context.matchBefore(/\\[a-zA-Z@]*/);
    if (command === null || (command.from === command.to && !context.explicit)) {
      return null;
    }

    return {
      // After the backslash, which stays.
      from: command.from + 1,
      options: commandOptions(at, command.text.slice(1)),
      validFor: /^[a-zA-Z@]*$/,
    };
  };
}

/**
 * How many of the commands nobody has loaded to offer.
 *
 * There are sixty thousand of them. They are a rescue for a command someone
 * knows they want and has forgotten the package for, so a handful of the
 * closest matches is the whole of their usefulness.
 */
const AVAILABLE_LIMIT = 5;

/**
 * Commands, filtered to what the typed prefix could still become.
 *
 * The filtering is the point. A document has around twenty-five thousand
 * commands defined and the corpus knows of sixty thousand more, and building a
 * completion object for every one of them on every keystroke is enough work to
 * feel it. Narrowing to the prefix first turns that into a few dozen.
 *
 * Returning a prefix-filtered set is sound because typing only ever adds
 * characters: everything matching a longer prefix already matches this one, so
 * `validFor` lets CodeMirror keep filtering without asking again.
 */
function commandOptions(at: CompletionSources, prefix: string): Completion[] {
  const wanted = prefix.toLowerCase();
  const matches = (name: string): boolean =>
    wanted === "" || name.toLowerCase().startsWith(wanted);

  const options: Completion[] = [];

  // The project's own definitions rank above everything: they are the ones
  // that cannot be looked up anywhere.
  const own = new Map(at.symbols.commands.map((command) => [command.name, command]));
  const defined = new Map(at.commands.map((command) => [command.name, command]));

  for (const [name, declared] of own) {
    if (!matches(name)) continue;

    // The engine knows the real arity once the document has compiled; before
    // that, what the declaration itself said is the best available answer.
    const merged = defined.get(name);
    const arity = merged?.arity ?? declared.arity ?? 0;
    const template = commandSnippet({
      name,
      tier: "bare",
      signatures: merged?.signatures ?? null,
      arity,
      optional: merged?.optional ?? false,
      classification: null,
      from: null,
    });

    const option: Completion = {
      label: name,
      type: "function",
      detail: `${declared.path}:${declared.line}`,
      boost: 2,
    };
    options.push(template === null ? option : snippetCompletion(template, option));
  }

  for (const command of at.commands) {
    if (own.has(command.name) || !matches(command.name)) continue;

    // Rarely-used commands exist and are worth completing if asked for by
    // name, but should not crowd out the ones people mean.
    const rare = command.classification?.unusual === true;
    const template = commandSnippet(command);
    const detail = describeSignature(command);

    const option: Completion = {
      label: command.name,
      type: command.tier === "described" ? "function" : "keyword",
      detail: detail === "" ? undefined : detail,
      boost: rare ? -2 : command.tier === "described" ? 1 : 0,
    };

    options.push(template === null ? option : snippetCompletion(template, option));
  }

  // Only now, over names rather than objects: the corpus knows of sixty
  // thousand commands nobody here has loaded, and the prefix rules out
  // essentially all of them. Stopping at the cap means the usual case walks a
  // few entries rather than the whole index.
  if (wanted !== "") {
    let available = 0;

    for (const name of at.providers.keys()) {
      if (available >= AVAILABLE_LIMIT) break;
      if (own.has(name) || defined.has(name) || !matches(name)) continue;

      const command = availableCommand(name, at.providers, defined);
      if (command === null) continue;

      available++;
      options.push(availableOption(command, at));
    }
  }

  return options;
}

/**
 * A command this document does not have, offered together with the package
 * that would give it.
 *
 * Accepting one writes the `\usepackage` as well, because a completion that
 * inserts a command the document cannot run is not help.
 */
function availableOption(command: MergedCommand, at: CompletionSources): Completion {
  const from = command.from;

  return {
    label: command.name,
    type: "interface",
    detail: from === null ? undefined : `\\usepackage{${from}}`,
    // Below everything that actually exists. These are a rescue for a command
    // someone knows they want, not a suggestion.
    boost: -50,
    apply: (view, _completion, start, end) => {
      view.dispatch({ changes: { from: start, to: end, insert: command.name } });
      if (from !== null) at.addPackage(from);
    },
  };
}

/** Environment names, from the corpus where there is one. */
function environmentNames(at: CompletionSources): string[] {
  const names = new Set<string>(at.symbols.environments.map((e) => e.name));
  for (const name of FALLBACK_ENVIRONMENTS) names.add(name);
  return [...names];
}

/**
 * Take the `}` that bracket-closing already supplied, if it is there.
 *
 * An environment snippet writes its own closing brace, a newline, a body and an
 * `\end`. Whatever was after the cursor ends up after all of that — so the `}`
 * the editor helpfully added when `\begin{` was typed would come out as a
 * second one on the `\end` line. Extending the replaced range over it is what
 * makes accepting a completion leave one brace where there should be one.
 */
function consumingBrace(completion: Completion): Completion {
  const apply = completion.apply;
  if (typeof apply !== "function") return completion;

  return {
    ...completion,
    apply: (view, described, from, to) => {
      const following = view.state.sliceDoc(to, to + 1) === "}" ? to + 1 : to;
      apply(view, described, from, following);
    },
  };
}

/** Environments, each writing its own `\end`. */
function environmentOptions(at: CompletionSources): Completion[] {
  const options: Completion[] = [];

  for (const declared of at.symbols.environments) {
    options.push(
      consumingBrace(
        snippetCompletion(environmentSnippet(declared.name), {
          label: declared.name,
          type: "class",
          detail: declared.path,
          boost: 2,
        }),
      ),
    );
  }

  const own = new Set(at.symbols.environments.map((e) => e.name));
  for (const name of FALLBACK_ENVIRONMENTS) {
    if (own.has(name)) continue;
    options.push(
      consumingBrace(
        snippetCompletion(environmentSnippet(name), { label: name, type: "class" }),
      ),
    );
  }

  return options;
}
