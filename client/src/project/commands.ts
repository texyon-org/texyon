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

import type { CwlArgument, CwlClassification, CwlPackage } from "./cwl";

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
 * `packages` are those the document loads, already fetched. `available` maps a
 * command name to a package that provides it, for the third tier — pass an
 * empty map to leave that out.
 */
export function mergeCommands(
  engine: readonly EngineCommand[],
  packages: readonly CwlPackage[],
  available: ReadonlyMap<string, string> = new Map(),
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
  const defined = new Set<string>();

  for (const command of engine) {
    defined.add(command.name);
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

  // Anything the corpus knows of that this document does not have. Offering it
  // as a completion would be wrong — it does not exist — but knowing which
  // package provides it is what lets the editor suggest loading one.
  for (const [name, from] of available) {
    if (defined.has(name)) continue;
    merged.push({
      name,
      tier: "available",
      signatures: null,
      arity: 0,
      optional: false,
      classification: null,
      from,
    });
  }

  return merged;
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
