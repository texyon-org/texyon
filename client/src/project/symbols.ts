/**
 * What a project defines, so the editor can offer it back.
 *
 * Autocomplete for LaTeX is only useful if it knows about *this* document:
 * offering `\section` is worth little, offering the label you wrote two files
 * ago is worth a lot. So the useful half of a completion source is an index of
 * what the project itself declares — labels, citation keys, custom commands and
 * environments — and that is what this builds.
 *
 * It is a scan, not a parse. A real TeX parser would have to expand macros to
 * know what a token means, and macro expansion is Turing-complete; an editor
 * offering suggestions cannot wait for that and does not need to. What it needs
 * is to find the declarations someone actually wrote, and those are written
 * plainly.
 */

/** A kind of thing a project can declare. */
export type SymbolKind = "label" | "citation" | "command" | "environment";

/** One thing the project declares. */
export interface ProjectSymbol {
  kind: SymbolKind;
  /** The name to complete — without the leading backslash for a command. */
  name: string;
  /** Where it was declared. */
  path: string;
  /** One-based line it was declared on. */
  line: number;
  /** How many arguments a command takes, where that was stated. */
  arity?: number;
}

/** Everything a project declares, by kind. */
export interface SymbolIndex {
  labels: ProjectSymbol[];
  citations: ProjectSymbol[];
  commands: ProjectSymbol[];
  environments: ProjectSymbol[];
}

/**
 * Comments are not declarations.
 *
 * A commented-out `\label` should not be offered, and an escaped `\%` does not
 * start a comment. Stripping per line is enough because TeX comments run to the
 * end of one.
 */
function withoutComment(line: string): string {
  let escaped = false;
  for (let index = 0; index < line.length; index++) {
    const character = line[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (character === "\\") {
      escaped = true;
      continue;
    }
    if (character === "%") return line.slice(0, index);
  }
  return line;
}

const LABEL = /\\label\s*\{([^}]+)\}/g;
const NEW_COMMAND = /\\(?:re)?newcommand\*?\s*\{?\\([A-Za-z@]+)\}?\s*(?:\[(\d+)\])?/g;
const NEW_ENVIRONMENT = /\\(?:re)?newenvironment\*?\s*\{([^}]+)\}\s*(?:\[(\d+)\])?/g;
const DECLARE_MATH = /\\DeclareMathOperator\*?\s*\{?\\([A-Za-z@]+)\}?/g;
// Every declaration above needs one of these, so only lines holding one are scanned.
const DECLARES = /\\label|newcommand|newenvironment|\\DeclareMathOperator/g;
// A BibTeX entry's key is whatever follows the opening brace up to the comma.
const BIB_ENTRY = /^\s*@(\w+)\s*\{\s*([^,\s}]+)\s*,/;

/** Build the index for a project's files. */
export function indexProject(files: Record<string, string | Uint8Array>): SymbolIndex {
  const index: SymbolIndex = {
    labels: [],
    citations: [],
    commands: [],
    environments: [],
  };

  for (const [path, text] of Object.entries(files)) {
    if (typeof text !== "string") continue;
    const lower = path.toLowerCase();
    if (lower.endsWith(".bib")) {
      collectCitations(path, text, index);
    } else if (lower.endsWith(".tex") || lower.endsWith(".sty") || lower.endsWith(".cls")) {
      collectTex(path, text, index);
    }
  }

  // Object.values over a heterogeneous interface widens to any; the lists are
  // named so the sort stays typed.
  for (const list of [index.labels, index.citations, index.commands, index.environments]) {
    list.sort((left, right) => left.name.localeCompare(right.name));
  }

  return index;
}

function collectTex(path: string, text: string, index: SymbolIndex): void {
  let at = 1;
  let counted = 0;
  let scannedTo = -1;

  for (const found of text.matchAll(DECLARES)) {
    if (found.index <= scannedTo) continue;
    const start = text.lastIndexOf("\n", found.index) + 1;
    const newline = text.indexOf("\n", found.index);
    const end = newline < 0 ? text.length : newline;
    // Line numbers are counted only up to each hit, not for every line.
    let next = text.indexOf("\n", counted);
    while (next !== -1 && next < start) {
      at++;
      next = text.indexOf("\n", next + 1);
    }
    counted = start;
    scannedTo = end;

    const line = withoutComment(text.slice(start, end));
    if (line === "") continue;

    for (const match of line.matchAll(LABEL)) {
      const name = match[1];
      if (name !== undefined) index.labels.push({ kind: "label", name, path, line: at });
    }

    for (const match of line.matchAll(NEW_COMMAND)) {
      const name = match[1];
      if (name === undefined) continue;
      const arity = match[2] === undefined ? undefined : Number(match[2]);
      index.commands.push({ kind: "command", name, path, line: at, arity });
    }

    for (const match of line.matchAll(DECLARE_MATH)) {
      const name = match[1];
      if (name !== undefined) index.commands.push({ kind: "command", name, path, line: at });
    }

    for (const match of line.matchAll(NEW_ENVIRONMENT)) {
      const name = match[1];
      if (name === undefined) continue;
      const arity = match[2] === undefined ? undefined : Number(match[2]);
      index.environments.push({ kind: "environment", name, path, line: at, arity });
    }
  }
}

function collectCitations(path: string, text: string, index: SymbolIndex): void {
  for (const [offset, line] of text.split("\n").entries()) {
    const match = BIB_ENTRY.exec(line);
    const name = match?.[2];
    // @string, @preamble and @comment declare no citation key.
    const type = match?.[1]?.toLowerCase();
    if (name === undefined || type === undefined) continue;
    if (type === "string" || type === "preamble" || type === "comment") continue;

    index.citations.push({ kind: "citation", name, path, line: offset + 1 });
  }
}
