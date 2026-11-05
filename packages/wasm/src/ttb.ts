/**
 * Reading Tectonic's own bundle format, TTBv1.
 *
 * A `.ttb` is a 66-byte header followed by concatenated gzip blobs, one of
 * which is the index. The index names every file with its byte range, and
 * carries the search rules that turn a bare TeX name like `article.cls` into
 * one of those paths. Everything here is pure parsing and resolution — the
 * worker does the fetching — which is also what makes it testable in Node.
 *
 * The resolution rules deliberately mirror `crates/bundles/src/ttb.rs`, quirks
 * included: the native engine is the semantics, and a file the CLI finds that
 * the browser does not (or the reverse) would be a miserable bug to chase.
 */

/** Where the index blob lives, read out of the fixed-size header. */
export interface TtbHeader {
  indexOffset: number;
  indexGzLength: number;
  indexRealLength: number;
}

/** One file's blob: where it starts, how long, and how long once inflated. */
export interface TtbLocation {
  offset: number;
  gzLength: number;
  realLength: number;
}

/** The header is always this long; fetch exactly this many bytes to parse it. */
export const TTB_HEADER_BYTES = 66;

const MAGIC = "tectonicbundle";

/**
 * Parse the fixed header at the front of a `.ttb`.
 *
 * Throws rather than returning null: every failure here means the URL is not a
 * TTBv1 bundle at all, and the caller's error path wants the reason.
 */
export function parseTtbHeader(bytes: Uint8Array): TtbHeader {
  if (bytes.length < TTB_HEADER_BYTES) {
    throw new Error(`a ttb header is ${TTB_HEADER_BYTES} bytes; got ${bytes.length}`);
  }

  for (let i = 0; i < MAGIC.length; i++) {
    if (bytes[i] !== MAGIC.charCodeAt(i)) {
      throw new Error("not a tectonic bundle: the magic bytes are wrong");
    }
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint32(14, true);
  if (version !== 1) {
    throw new Error(`ttb version ${version} is not supported; only version 1 is`);
  }

  // The offset is a u64. Number.MAX_SAFE_INTEGER is 8 petabytes as a byte
  // count, so getBigUint64 followed by a checked narrowing is plenty.
  const offset = view.getBigUint64(18, true);
  if (offset > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error("the bundle index offset does not fit in a number");
  }

  return {
    indexOffset: Number(offset),
    indexGzLength: view.getUint32(26, true),
    indexRealLength: view.getUint32(30, true),
  };
}

interface Entry {
  path: string;
  location: TtbLocation;
}

/**
 * The parsed index: every file's location, and the search rules that decide
 * which of several same-named files a bare name means.
 */
export class TtbIndex {
  #byBasename = new Map<string, Entry[]>();
  #search: string[] = [];
  #resolved = new Map<string, TtbLocation | null>();
  #size = 0;

  /**
   * Parse the decompressed index text.
   *
   * Sections open with a `[NAME]` line. `[FILELIST]` lines are
   * `<start> <gzip_len> <real_len> <hash> <path>` — the path last, because it
   * may contain spaces. Unknown sections are ignored, as the spec requires.
   */
  static parse(text: string): TtbIndex {
    const index = new TtbIndex();

    const searchOrders = new Map<string, string[]>();
    let defaultSearch = "";
    let section = "";
    let sectionArg = "";

    for (const line of text.split("\n")) {
      if (line === "") continue;

      if (line.startsWith("[") && line.endsWith("]")) {
        const name = line.slice(1, -1);
        const colon = name.indexOf(":");
        section = colon === -1 ? name : name.slice(0, colon);
        sectionArg = colon === -1 ? "" : name.slice(colon + 1);
        continue;
      }

      switch (section) {
        case "DEFAULTSEARCH":
          // Only the last line has any effect, says the spec.
          defaultSearch = line;
          break;

        case "SEARCH": {
          let rules = searchOrders.get(sectionArg);
          if (!rules) {
            rules = [];
            searchOrders.set(sectionArg, rules);
          }
          rules.push(line);
          break;
        }

        case "FILELIST": {
          const entry = parseFileLine(line);
          if (entry === null) continue;

          const basename = entry.path.slice(entry.path.lastIndexOf("/") + 1);
          let list = index.#byBasename.get(basename);
          if (!list) {
            list = [];
            index.#byBasename.set(basename, list);
          }
          list.push(entry);
          index.#size++;
          break;
        }

        default:
          // An unknown section, skipped whole.
          break;
      }
    }

    const search = searchOrders.get(defaultSearch);
    if (!search) {
      // The native reader unwraps here; an index without its default search
      // order is broken, not merely sparse.
      throw new Error(`the bundle index names no [SEARCH:${defaultSearch}] section`);
    }
    index.#search = search;

    return index;
  }

  /** How many files the bundle holds. */
  get size(): number {
    return this.#size;
  }

  /**
   * The location a name means, under the bundle's search rules.
   *
   * A bare name is matched by basename and disambiguated by the search order —
   * first rule with any match wins, alphabetically first path among ties. A
   * name with a slash is a relative path and must match exactly one entry's
   * tail. An absolute name never matches; nothing in a bundle is absolute.
   */
  resolve(name: string): TtbLocation | null {
    const cached = this.#resolved.get(name);
    if (cached !== undefined) return cached;

    const found = this.#search_(name);
    this.#resolved.set(name, found);
    return found;
  }

  #search_(name: string): TtbLocation | null {
    if (name.startsWith("/")) return null;

    const basename = name.slice(name.lastIndexOf("/") + 1);
    const candidates = this.#byBasename.get(basename);
    if (!candidates) return null;

    if (basename !== name) {
      // A relative path. It must pin down exactly one file; two entries both
      // ending in it is a bundle ambiguity the native reader also refuses.
      let matching: Entry | null = null;
      for (const entry of candidates) {
        if (entry.path.endsWith(name)) {
          if (matching !== null) return null;
          matching = entry;
        }
      }
      return matching?.location ?? null;
    }

    // A bare name: the first search rule with any match decides.
    for (const rule of this.#search) {
      // Rules start with a slash; paths do not.
      const bare = rule.slice(1);

      let picked: Entry[] = [];
      for (const entry of candidates) {
        if (bare.endsWith("//")) {
          // Recursive: match anywhere under the prefix (final slash dropped).
          if (entry.path.startsWith(bare.slice(0, -1))) picked.push(entry);
        } else if (entry.path.slice(0, entry.path.length - name.length) === bare) {
          // Exact: the file's parent directory is the rule, nothing deeper.
          picked.push(entry);
        }
      }

      if (picked.length > 0) {
        // Same priority; alphabetic order stands in for the DFS the native
        // resolver emulates.
        picked = picked.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
        return picked[0].location;
      }
    }

    return null;
  }
}

/** One `[FILELIST]` line: four fields, then a path that may contain spaces. */
function parseFileLine(line: string): Entry | null {
  const first = line.indexOf(" ");
  const second = line.indexOf(" ", first + 1);
  const third = line.indexOf(" ", second + 1);
  const fourth = line.indexOf(" ", third + 1);
  if (first < 0 || second < 0 || third < 0 || fourth < 0) return null;

  const offset = Number(line.slice(0, first));
  const gzLength = Number(line.slice(first + 1, second));
  const realLength = Number(line.slice(second + 1, third));
  const path = line.slice(fourth + 1);

  if (!Number.isFinite(offset) || !Number.isFinite(gzLength) || !Number.isFinite(realLength)) {
    return null;
  }
  if (path === "") return null;

  return { path, location: { offset, gzLength, realLength } };
}
