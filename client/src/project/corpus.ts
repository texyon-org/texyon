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
  return resolvePackages(packagesUsed(source), loadPackage);
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
