/**
 * What the editor knows about a command, from both places that know something.
 *
 * The engine's macro table says what *this* document has defined and how many
 * arguments each takes — correct for the packages actually loaded, under the
 * options they were actually given, including anything the preamble built at
 * run time. What it cannot say is what those arguments are called, where a
 * command is legal, or which package would provide one that is missing, because
 * TeX does not record any of that.
 *
 * The CWL corpus knows exactly those things and nothing about this document.
 * So the two are merged rather than ranked against each other: the engine
 * decides what exists, the corpus decides what it means.
 */

import type { CwlArgument, CwlClassification, CwlKey, CwlPackage } from "./cwl";

/** One command the engine reports as defined. */
export interface EngineCommand {
  name: string;
  /** Arguments counted from the macro's parameter text. */
  arity: number;
  /** Whether the first is optional. */
  optional: boolean;
  kind: "macro" | "protected" | "builtin";
}

/** How much is known about a command. */
export type CommandTier =
  /** Defined here, and the corpus describes it. Everything is known. */
  | "described"
  /** Defined here, but no corpus entry. Arity only. */
  | "bare"
  /** Not defined here; a package that would provide it is known. */
  | "available";

/** A command, as the editor will offer it. */
export interface MergedCommand {
  name: string;
  tier: CommandTier;
  /**
   * Every way the corpus says the command may be written, in the order it
   * listed them.
   *
   * More than one is normal rather than exceptional: `\section` has four —
   * with and without a star, with and without a short title — and an editor
   * that kept only the first would silently drop the optional argument. Null
   * when the corpus does not describe the command at all.
   */
  signatures: CwlArgument[][] | null;
  /** Argument count, from whichever source had one. */
  arity: number;
  /** Whether the first argument is optional. */
  optional: boolean;
  classification: CwlClassification | null;
  /** Which package to load, for an `available` command. */
  from: string | null;
}

/**
 * Merge what the engine has with what the corpus describes.
 *
 * `packages` are those the document loads, already fetched.
 *
 * The third tier — commands no package here provides — is deliberately *not*
 * built. There are sixty thousand of them against twenty-five thousand that
 * exist, and materialising an object for each on every compile was most of the
 * work this function did. They are a lookup by name, so
 * {@link availableCommand} answers for one at the moment something asks.
 */
export function mergeCommands(
  engine: readonly EngineCommand[],
  packages: readonly CwlPackage[],
): MergedCommand[] {
  const described = new Map<
    string,
    { signatures: CwlArgument[][]; classification: CwlClassification }
  >();

  for (const pkg of packages) {
    for (const entry of pkg.commands) {
      const existing = described.get(entry.name);
      if (existing === undefined) {
        described.set(entry.name, {
          signatures: [entry.arguments],
          classification: entry.classification,
        });
        continue;
      }
      // Another way of writing the same command, which the corpus lists as its
      // own line. Keep it, unless some package has already said the same thing.
      const shape = JSON.stringify(entry.arguments);
      if (!existing.signatures.some((known) => JSON.stringify(known) === shape)) {
        existing.signatures.push(entry.arguments);
      }
    }
  }

  const merged: MergedCommand[] = [];

  for (const command of engine) {
    const corpus = described.get(command.name);

    if (corpus === undefined) {
      merged.push({
        name: command.name,
        tier: "bare",
        signatures: null,
        arity: command.arity,
        optional: command.optional,
        classification: null,
        from: null,
      });
      continue;
    }

    // The corpus is trusted for argument *names*, and for the count where the
    // engine could not recover one — \section and friends are built from
    // machinery that reads the input stream, so their parameter text is empty
    // and the engine honestly reports zero.
    // Of the ways it may be written, the fullest is the one to describe it by:
    // it is the only one that shows every argument the command can take.
    const fullest = corpus.signatures.reduce((widest, candidate) =>
      candidate.length > widest.length ? candidate : widest,
    );

    merged.push({
      name: command.name,
      tier: "described",
      signatures: corpus.signatures,
      arity: command.arity > 0 ? command.arity : fullest.length,
      optional: command.arity > 0 ? command.optional : (fullest[0]?.optional ?? false),
      classification: corpus.classification,
      from: null,
    });
  }

  return merged;
}

/** Whatever the caller already keeps its defined names in. */
export interface HasName {
  has(name: string): boolean;
}

/**
 * A command this document does not have, described by the package that would
 * provide it.
 *
 * Returns null when the corpus does not know the name, or when the document
 * already has it — a command that exists is not something to go and load.
 */
export function availableCommand(
  name: string,
  providers: ReadonlyMap<string, string>,
  defined: HasName,
): MergedCommand | null {
  if (defined.has(name)) return null;

  const from = providers.get(name);
  if (from === undefined) return null;

  return {
    name,
    tier: "available",
    signatures: null,
    arity: 0,
    optional: false,
    classification: null,
    from,
  };
}

/**
 * The packages a document loads, in the order they should be fetched.
 *
 * Read from the source text rather than from the engine, because the engine
 * has no idea which package defined what — and because this has to work before
 * the first compile, when there is no macro table to consult at all.
 *
 * Transitive loads are followed through the corpus's own `#include:` lines,
 * which is how `\usepackage{tikz}` reaches `pgf` and everything under it.
 */
export function packagesUsed(source: string): string[] {
  const found = new Set<string>();

  // Every LaTeX document has these whether it says so or not.
  found.add("latex-document");
  found.add("tex");

  for (const match of source.matchAll(/\\documentclass\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/g)) {
    const name = match[1]?.trim();
    // Classes are filed under a prefix in the corpus.
    if (name !== undefined && name !== "") found.add(`class-${name}`);
  }

  // \usepackage and \RequirePackage both load, and both accept a comma-separated
  // list in one call.
  for (const match of source.matchAll(
    /\\(?:usepackage|RequirePackage)\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/g,
  )) {
    for (const name of match[1]?.split(",") ?? []) {
      const trimmed = name.trim();
      if (trimmed !== "") found.add(trimmed);
    }
  }

  return [...found];
}

/**
 * Follow `#include:` from a set of packages to everything they pull in.
 *
 * `load` returns null for a package the corpus does not describe, which is
 * ordinary: it covers a few thousand of CTAN's many more.
 */
export async function resolvePackages(
  names: readonly string[],
  load: (name: string) => Promise<CwlPackage | null>,
): Promise<CwlPackage[]> {
  const seen = new Set<string>();
  const resolved: CwlPackage[] = [];
  const queue = [...names];

  while (queue.length > 0) {
    const name = queue.shift();
    if (name === undefined || seen.has(name)) continue;
    seen.add(name);

    const pkg = await load(name);
    if (pkg === null) continue;

    resolved.push(pkg);
    queue.push(...pkg.includes);
  }

  return resolved;
}

/** One environment the corpus describes, ready to offer. */
export interface MergedEnvironment {
  name: string;
  /**
   * What goes after `\begin{name}`, from the richest description found.
   *
   * The corpus lists an environment once per way of writing it — tikz declares
   * `tikzpicture` both bare and with `[options]` — so keeping the longest is
   * what makes accepting the completion leave you somewhere useful.
   */
  arguments: readonly CwlArgument[];
  classification: CwlClassification | null;
  /** The package that describes it. */
  from: string;
}

/**
 * Every environment the packages this document loads contribute.
 *
 * The corpus has always carried these — `parseCwl` reads every `\begin{…}`
 * line and the build ships them — but nothing offered them, so `\begin{`
 * knew only what the project declared itself and a short hardcoded list.
 * `tikzpicture`, beamer's `frame` and `tabularx` were all missing for that
 * reason rather than any deeper one.
 */
export function mergeEnvironments(packages: readonly CwlPackage[]): MergedEnvironment[] {
  const best = new Map<string, MergedEnvironment>();

  for (const pkg of packages) {
    for (const entry of pkg.environments) {
      const existing = best.get(entry.name);
      if (existing !== undefined && existing.arguments.length >= entry.arguments.length) {
        continue;
      }

      best.set(entry.name, {
        name: entry.name,
        arguments: entry.arguments,
        classification: entry.classification,
        from: pkg.name,
      });
    }
  }

  return [...best.values()];
}

/**
 * Every key-value option the loaded packages describe, by what accepts it.
 *
 * The owner is the corpus's own spelling: a command by bare name (`draw`,
 * `includegraphics`), an environment as `begin{name}`, and a package's own
 * load-time options as `usepackage/name`.
 *
 * These come from packages already in memory, so looking one up costs nothing
 * — unlike `\usepackage[…]{name}`, where the package being configured is by
 * definition not loaded yet and has to be fetched.
 */
export function mergeKeys(packages: readonly CwlPackage[]): Map<string, CwlKey[]> {
  // Indexed by name rather than searched. The corpus repeats a key once per
  // way of writing its value — tikz lists `baseline` bare and `baseline=` —
  // so this deduplicates, and scanning the list for each of pgfplots' 7,187
  // keys to do it is quadratic: 12.8 ms for one document's worth, and worse
  // the more it knows.
  const merged = new Map<string, Map<string, CwlKey>>();

  for (const pkg of packages) {
    for (const [owner, keys] of Object.entries(pkg.keys)) {
      let byName = merged.get(owner);
      if (byName === undefined) merged.set(owner, (byName = new Map()));

      for (const key of keys) {
        const existing = byName.get(key.name);
        // Keep whichever says most about its value.
        if (existing === undefined || describes(key) > describes(existing)) {
          byName.set(key.name, key);
        }
      }
    }
  }

  return new Map([...merged].map(([owner, byName]) => [owner, [...byName.values()]]));
}

/** How much a key says about its value, for choosing between duplicates. */
function describes(key: CwlKey): number {
  if (key.values.length > 0) return 2;
  if (key.hint !== null) return 1;
  return 0;
}
