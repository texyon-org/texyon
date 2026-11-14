/**
 * Reader for TTBv1 bundles: a 66-byte header, then gzip blobs, one of them the
 * index. Resolution mirrors `crates/bundles/src/ttb.rs`.
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

/** Length of the fixed header. */
export const TTB_HEADER_BYTES = 66;

/** Part of the TTBv1 file format. */
const MAGIC = "tectonicbundle";

/** Parse the fixed header at the front of a `.ttb`. */
export function parseTtbHeader(bytes: Uint8Array): TtbHeader {
  if (bytes.length < TTB_HEADER_BYTES) {
    throw new Error(`a ttb header is ${TTB_HEADER_BYTES} bytes; got ${bytes.length}`);
  }

  for (let i = 0; i < MAGIC.length; i++) {
    if (bytes[i] !== MAGIC.charCodeAt(i)) {
      throw new Error("not a ttb bundle: the magic bytes are wrong");
    }
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint32(14, true);
  if (version !== 1) {
    throw new Error(`ttb version ${version} is not supported; only version 1 is`);
  }

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

/** A parsed bundle index with its search rules. */
export class TtbIndex {
  #byBasename = new Map<string, Entry[]>();
  #search: string[] = [];
  #resolved = new Map<string, TtbLocation | null>();
  #size = 0;

  /** Parse the decompressed index text. */
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

      }
    }

    const search = searchOrders.get(defaultSearch);
    if (!search) {
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
   * The file a name means. A bare name goes through the search rules, the first
   * rule with a match winning and the alphabetically first path among ties. A
   * relative path must match the tail of exactly one entry.
   */
  resolve(name: string): TtbLocation | null {
    const cached = this.#resolved.get(name);
    if (cached !== undefined) return cached;

    const found = this.#find(name);
    this.#resolved.set(name, found);
    return found;
  }

  #find(name: string): TtbLocation | null {
    if (name.startsWith("/")) return null;

    const basename = name.slice(name.lastIndexOf("/") + 1);
    const candidates = this.#byBasename.get(basename);
    if (!candidates) return null;

    if (basename !== name) {
      let matching: Entry | null = null;
      for (const entry of candidates) {
        if (entry.path.endsWith(name)) {
          if (matching !== null) return null;
          matching = entry;
        }
      }
      return matching?.location ?? null;
    }

    for (const rule of this.#search) {
      const bare = rule.slice(1);

      const picked: Entry[] = [];
      for (const entry of candidates) {
        if (bare.endsWith("//")) {
          if (entry.path.startsWith(bare.slice(0, -1))) picked.push(entry);
        } else if (entry.path.slice(0, entry.path.length - name.length) === bare) {
          picked.push(entry);
        }
      }

      if (picked.length > 0) {
        picked.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
        return picked[0].location;
      }
    }

    return null;
  }
}

/** A `[FILELIST]` line: offset, gzip length, real length, hash, then a path that may hold spaces. */
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
