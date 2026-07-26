/**
 * Reading TeXstudio's completion word lists.
 *
 * The engine knows which commands a document has and how many arguments they
 * take; what it cannot know is what those arguments are *called*, where a
 * command is legal, or which package would provide one that is missing. None
 * of that is in TeX — it is editorial knowledge, and the CWL corpus is twenty
 * years of it.
 *
 * This reads the format rather than the corpus: the licence question over the
 * data is unresolved, so the files are fetched at build time and never vendored
 * here. See docs/editor/completion.md.
 *
 * The format is line-based. A line beginning with a backslash declares a
 * command or environment; a line beginning with `#` is either a directive or a
 * comment; everything else inside a `#keyvals:` block is a key.
 */

/** Where a command may be used, and whether it is worth offering. */
export interface CwlClassification {
  /** `#*` — rarely used; offer only when asked for everything. */
  unusual: boolean;
  /** `#S` — do not offer at all. */
  hidden: boolean;
  /** `#m` — maths only. */
  mathOnly: boolean;
  /** `#n` — text only, never maths. */
  textOnly: boolean;
  /** `#t` — inside a tabular. */
  tabularOnly: boolean;
  /** `#r` reference, `#c`/`#C` citation, `#l` label, `#d` definition, `#L`
   *  length, `#B` colour. What kind of name the argument expects. */
  role: "ref" | "cite" | "label" | "definition" | "length" | "colour" | null;
  /** Environments it is restricted to, from `#\math,array`. */
  environments: string[];
}

/** One argument of a command. */
export interface CwlArgument {
  /** Whether it is `{braced}` or `[bracketed]`. */
  optional: boolean;
  /** What the corpus calls it — `title`, `file`, `ncols`. */
  name: string;
  /**
   * What sort of thing goes in it, where the corpus says.
   *
   * A `%` suffix on an argument annotates its type rather than naming it:
   * `h-length%l` is a length called `h-length`, `code%plain` is verbatim text.
   * Null where nothing was said.
   */
  hint: string | null;
  /** Whether it takes `key=value` pairs rather than a value. */
  keyvals: boolean;
}

/** One command or environment the corpus describes. */
export interface CwlEntry {
  /** Without the leading backslash; for an environment, its bare name. */
  name: string;
  arguments: CwlArgument[];
  classification: CwlClassification;
}

/** One key accepted inside a key-value argument. */
export interface CwlKey {
  name: string;
  /** Values it accepts, where the corpus enumerates them. */
  values: string[];
  /** Whether it takes a value at all — `draft` does not. */
  takesValue: boolean;
  /**
   * What kind of value it wants, where the corpus says so rather than listing
   * them — `%color`, `%length`. A hint to show, not a value to insert.
   */
  hint: string | null;
}

/** Everything one package contributes. */
export interface CwlPackage {
  name: string;
  /** Packages it loads, which contribute their commands too. */
  includes: string[];
  commands: CwlEntry[];
  environments: CwlEntry[];
  /** Keys, by the command that accepts them. */
  keys: Record<string, CwlKey[]>;
}

const EMPTY_CLASSIFICATION: CwlClassification = {
  unusual: false,
  hidden: false,
  mathOnly: false,
  textOnly: false,
  tabularOnly: false,
  role: null,
  environments: [],
};

/** Parse one `.cwl` file. */
export function parseCwl(name: string, source: string): CwlPackage {
  const pkg: CwlPackage = {
    name,
    includes: [],
    commands: [],
    environments: [],
    keys: {},
  };

  /** The command whose keys are currently being collected, if inside a block. */
  let keysFor: string | null = null;

  for (const raw of source.split("\n")) {
    const line = raw.trim();
    if (line === "") continue;

    if (keysFor !== null) {
      if (line === "#endkeyvals") {
        keysFor = null;
      } else if (!line.startsWith("#")) {
        (pkg.keys[keysFor] ??= []).push(parseKey(line));
      }
      continue;
    }

    if (line.startsWith("#")) {
      const include = line.match(/^#include:(.+)$/);
      if (include?.[1] !== undefined) {
        pkg.includes.push(include[1].trim());
        continue;
      }

      const keyvals = line.match(/^#keyvals:(.+)$/);
      if (keyvals?.[1] !== undefined) {
        // A block may serve several commands at once, comma-separated, and
        // each name may carry its own classification suffix.
        for (const owner of keyvals[1].split(",")) {
          const command = owner.trim().replace(/#.*$/, "").replace(/^\\/, "");
          if (command !== "") keysFor = command;
        }
        // `#ifOption` blocks are taken unconditionally: which options a
        // document passed is not known here, and a key that turns out not to
        // apply is a worse suggestion than no suggestion but not a wrong one.
      }
      continue;
    }

    if (!line.startsWith("\\")) continue;

    const entry = parseEntry(line);
    if (entry === null) continue;

    if (entry.environment) {
      pkg.environments.push(entry.value);
    } else {
      pkg.commands.push(entry.value);
    }
  }

  return pkg;
}

/** Parse one command or environment line. */
function parseEntry(line: string): { value: CwlEntry; environment: boolean } | null {
  const { body, suffix } = splitClassification(line);

  const environment = body.match(/^\\begin\{([^}]+)\}/);
  if (environment?.[1] !== undefined) {
    return {
      environment: true,
      value: {
        name: environment[1],
        // Anything after `\begin{env}` is the environment's own arguments.
        arguments: parseArguments(body.slice(environment[0].length)),
        classification: parseClassification(suffix),
      },
    };
  }

  // `\end{...}` adds nothing: the environment is already declared by its
  // `\begin`, and offering both would double every entry.
  if (body.startsWith("\\end{")) return null;

  const command = body.match(/^\\([a-zA-Z@]+\*?|.)/);
  if (command?.[1] === undefined) return null;

  return {
    environment: false,
    value: {
      name: command[1],
      arguments: parseArguments(body.slice(command[0].length)),
      classification: parseClassification(suffix),
    },
  };
}

/**
 * Split a line into its declaration and its trailing classification.
 *
 * The `#` that starts a classification is the last one outside any argument —
 * `\newcommand{cmd}{def}#d` classifies, but a `#` inside braces is part of an
 * argument's own text.
 */
function splitClassification(line: string): { body: string; suffix: string } {
  let depth = 0;
  for (let index = 0; index < line.length; index++) {
    const character = line[index];
    if (character === "{" || character === "[") depth++;
    else if (character === "}" || character === "]") depth--;
    else if (character === "#" && depth <= 0) {
      return { body: line.slice(0, index), suffix: line.slice(index + 1) };
    }
  }
  return { body: line, suffix: "" };
}

/** Read the `{…}` and `[…]` groups that follow a command name. */
function parseArguments(rest: string): CwlArgument[] {
  const found: CwlArgument[] = [];

  for (const match of rest.matchAll(/\{([^{}]*)\}|\[([^\][]*)\]/g)) {
    const braced = match[1];
    const bracketed = match[2];
    // `%<…%>` wraps a placeholder and `%|` marks a cursor stop; neither is part
    // of what the argument is called.
    const inner = (braced ?? bracketed ?? "").replace(/%<|%>|%\|/g, "");

    // Anything from a remaining `%` on annotates the argument's type rather
    // than naming it — `h-length%l`, `num%:translatable`, `options%keyvals`.
    const marker = inner.indexOf("%");
    const hint = marker < 0 ? null : inner.slice(marker + 1).replace(/^:/, "").trim();

    found.push({
      optional: braced === undefined,
      name: (marker < 0 ? inner : inner.slice(0, marker)).trim(),
      hint: hint === null || hint === "" || hint === "keyvals" ? null : hint,
      keyvals: hint === "keyvals",
    });
  }

  return found;
}

/** Read the letters after `#`. */
function parseClassification(suffix: string): CwlClassification {
  if (suffix === "") return { ...EMPTY_CLASSIFICATION, environments: [] };

  // `#\math,array` restricts to named environments rather than classifying.
  const environments = suffix.startsWith("\\")
    ? suffix
        .slice(1)
        .split(",")
        .map((name) => name.trim())
        .filter((name) => name !== "")
    : [];

  const letters = environments.length > 0 ? "" : suffix;

  const role = letters.includes("r")
    ? "ref"
    : letters.includes("c") || letters.includes("C")
      ? "cite"
      : letters.includes("l")
        ? "label"
        : letters.includes("d")
          ? "definition"
          : letters.includes("L")
            ? "length"
            : letters.includes("B")
              ? "colour"
              : null;

  return {
    unusual: letters.includes("*"),
    hidden: letters.includes("S"),
    mathOnly: letters.includes("m"),
    textOnly: letters.includes("n"),
    tabularOnly: letters.includes("t"),
    role,
    environments,
  };
}

/**
 * The `##` codes that stand for a length. Both spellings occur.
 */
const LENGTH_CODES = new Set(["L", "l"]);

/**
 * Read one line of a `#keyvals:` block.
 *
 * Everything after the first `#` describes the value, in one of three ways:
 *
 * - `clip#true,false` and `backref=#section,page,none` enumerate it. The `=`
 *   is notation — both are written `key=value` — and is not the difference
 *   between a flag and a key.
 * - `width=##L` and `citecolor=#%color` name a *kind* of value instead. A
 *   hint to show, never a value to insert.
 * - `baseurl=%<URL%>` has no `#` at all and names its value in the
 *   placeholder syntax used everywhere else in the corpus.
 *
 * A line with no value notation at all — a bare `draft` — is a flag.
 */
function parseKey(line: string): CwlKey {
  // Deliberately `[^#]`, not `[^#=]`. Excluding the `=` made this miss every
  // `key=#a,b,c` in the corpus — 5,601 of them — and fall through to the
  // branch below, which kept the name and dropped the values.
  const described = line.match(/^([^#]+)#(.+)$/);
  if (described?.[1] !== undefined && described[2] !== undefined) {
    const name = described[1].replace(/=$/, "").trim();
    const value = described[2].trim();

    // `##L` is a length and `#%color` is a colour: kinds, not choices.
    if (value.startsWith("#")) {
      return { name, values: [], takesValue: true, hint: LENGTH_CODES.has(value.slice(1)) ? "length" : null };
    }
    if (value.startsWith("%")) {
      return { name, values: [], takesValue: true, hint: value.slice(1) || null };
    }

    return {
      name,
      values: value.split(",").map((one) => one.trim()),
      takesValue: true,
      hint: null,
    };
  }

  const assigned = line.match(/^([^=]+)=(.*)$/);
  if (assigned?.[1] !== undefined) {
    return { name: assigned[1].trim(), values: [], takesValue: true, hint: placeholder(assigned[2]) };
  }

  return { name: line.trim(), values: [], takesValue: false, hint: null };
}

/** The name inside a `%<…%>` placeholder, if that is all the value is. */
function placeholder(value: string): string | null {
  const found = value.trim().match(/^%<(.+)%>$/);
  return found?.[1] ?? null;
}
