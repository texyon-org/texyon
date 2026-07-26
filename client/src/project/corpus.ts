/**
 * Fetching the parts of the corpus a document actually needs.
 *
 * All of it is about sixty megabytes; a document uses a handful of kilobytes of
 * it. So packages are fetched as the document turns out to load them, and the
 * index of which package provides which command — the largest single file — is
 * left until something asks a question only it can answer.
 */

import type { CwlPackage } from "./cwl";
import { packagesUsed, resolvePackages } from "./commands";

/** Where the build script put the converted corpus. */
const ROOT = "cwl/";

/** Packages already fetched, and the misses, which are worth remembering. */
const packages = new Map<string, CwlPackage | null>();

/** Command name to the package that best provides it, once it has been needed. */
let index: Map<string, string> | null = null;
let indexPending: Promise<void> | null = null;

/** Every package and class there is, once something has asked for the list. */
let catalogue: Promise<PackageCatalogue> | null = null;

/** What there is to load, ranked with the most depended-upon first. */
export interface PackageCatalogue {
  /** Names for `\usepackage{…}`. */
  packages: readonly string[];
  /** Names for `\documentclass{…}`, with the corpus's `class-` prefix gone. */
  classes: readonly string[];
}

const NOTHING: PackageCatalogue = { packages: [], classes: [] };

async function loadPackage(name: string): Promise<CwlPackage | null> {
  const known = packages.get(name);
  if (known !== undefined) return known;

  try {
    const response = await fetch(`${ROOT}${encodeURIComponent(name)}.json`);
    // A package the corpus never described is the ordinary case: it covers a
    // few thousand of CTAN's many more.
    const pkg = response.ok ? ((await response.json()) as CwlPackage) : null;
    packages.set(name, pkg);
    return pkg;
  } catch {
    packages.set(name, null);
    return null;
  }
}

/**
 * Everything the corpus knows about the packages a document loads.
 *
 * Read from the source text rather than from the engine, because the engine has
 * no idea which package defined what — and because this has to work before the
 * first compile, when there is no macro table to consult.
 */
export async function corpusFor(source: string): Promise<CwlPackage[]> {
  return corpusOf(packagesUsed(source));
}

/**
 * The corpus entries for an already-determined package list.
 *
 * Taking the list rather than the source is what lets a caller notice that
 * typing a sentence did not change which packages are loaded, and skip the
 * whole resolution.
 */
export async function corpusOf(packages: readonly string[]): Promise<CwlPackage[]> {
  return resolvePackages(packages, loadPackage);
}

/**
 * One package's entry, fetched if it has not been already.
 *
 * Unlike {@link corpusOf} this does not resolve what the package itself loads:
 * the caller wants what this one file says — its options — rather than
 * everything reachable from it.
 */
export async function packageEntry(name: string): Promise<CwlPackage | null> {
  return loadPackage(name);
}

/**
 * Every package and class there is, for completing `\usepackage{`.
 *
 * Fifty-six kilobytes, so it is fetched when someone first opens a brace after
 * `\usepackage` and not as part of starting the editor. Awaited rather than
 * answered emptily: a completion source that returns nothing has told
 * CodeMirror there is nothing to offer, and with a `validFor` it will go on
 * filtering that nothing rather than asking again.
 */
export async function catalogueOfPackages(): Promise<PackageCatalogue> {
  catalogue ??= (async () => {
    try {
      const response = await fetch(`${ROOT}packages.json`);
      if (!response.ok) return NOTHING;
      return (await response.json()) as PackageCatalogue;
    } catch {
      // No catalogue means no package-name completion; the rest still works.
      return NOTHING;
    }
  })();

  return catalogue;
}

/**
 * Which package provides a command, for commands this document does not have.
 *
 * The index is six hundred kilobytes and answers only this one question, so it
 * is fetched the first time that question is asked and not before. Until it
 * arrives this is empty, which costs a suggestion rather than correctness.
 *
 * The corpus lists several providers for a name — five packages define `\SI` —
 * ordered with the most depended-upon first, so the first is the one to offer.
 */
export function providers(): ReadonlyMap<string, string> {
  if (index !== null) return index;

  indexPending ??= (async () => {
    try {
      const response = await fetch(`${ROOT}index.json`);
      if (!response.ok) return;
      const raw = (await response.json()) as Record<string, string[]>;

      const found = new Map<string, string>();
      for (const [name, owners] of Object.entries(raw)) {
        const best = owners[0];
        if (best !== undefined) found.set(name, best);
      }
      index = found;
    } catch {
      // No index means no package suggestions; everything else still works.
    }
  })();

  return new Map();
}
