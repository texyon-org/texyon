/**
 * Keeping the engine's assets across page loads.
 *
 * A cold start costs about thirty megabytes: the module and the support pack.
 * None of it changes between loads, and re-fetching all of it every time is
 * the single largest thing standing between a reload and a working editor.
 *
 * Two kinds of asset are cached, and they are invalidated differently because
 * they change differently:
 *
 * - **The module and the pack** are revalidated with the server's own
 *   validators, so one conditional request each decides whether the cached
 *   copy still stands. A rebuilt engine is picked up on the next load without
 *   anyone bumping a version.
 * - **Bundle files** are keyed by the bundle they came from. A bundle tar is a
 *   published artifact and does not change under its own URL, so a file read
 *   out of one never needs checking again.
 *
 * Everything here fails soft. Storage can be disabled, full, or absent — in a
 * private window, under a strict policy, in a runtime with no Cache API — and
 * none of that is a reason to refuse to typeset. A cache that cannot be opened
 * simply is not used.
 */

/** The part of the Cache API this needs. */
export interface CacheLike {
  match(request: string): Promise<Response | undefined>;
  put(request: string, response: Response): Promise<void>;
  keys(): Promise<readonly { url: string }[]>;
}

/** The part of `caches` this needs. */
export interface CacheStorageLike {
  open(name: string): Promise<CacheLike>;
  keys(): Promise<string[]>;
  delete(name: string): Promise<boolean>;
}

/**
 * Name of the cache this version of the code writes.
 *
 * Bumping it abandons everything written by an older shape of these keys, which
 * is the cheapest way to change that shape without having to migrate anything.
 */
const CACHE_NAME = "tectonic-assets-v2";

/** Prefix of every cache this code has ever written, for cleaning up after itself. */
const CACHE_PREFIX = "tectonic-assets-";

/**
 * Keys are URLs because the Cache API insists, but nothing ever requests them
 * over the network. This origin does not exist and is never contacted.
 */
const KEY_ORIGIN = "https://tectonic.invalid/";

/** The engine's assets, held across page loads. */
export class AssetCache {
  #cache: CacheLike;

  private constructor(cache: CacheLike) {
    this.#cache = cache;
  }

  /**
   * Open the cache, discarding anything an older version of this code left.
   *
   * Resolves to `null` when there is no usable storage, which callers should
   * treat as "fetch everything" rather than as a failure.
   */
  static async open(storage: CacheStorageLike | undefined): Promise<AssetCache | null> {
    if (!storage) return null;

    try {
      const cache = await storage.open(CACHE_NAME);

      // Names from older versions of this file are unreachable now and would
      // otherwise sit in the origin's quota forever.
      const stale = (await storage.keys()).filter(
        (name) => name.startsWith(CACHE_PREFIX) && name !== CACHE_NAME,
      );
      await Promise.all(stale.map((name) => storage.delete(name)));

      return new AssetCache(cache);
    } catch {
      return null;
    }
  }

  /**
   * Fetch a URL, revalidating a cached copy rather than re-downloading it.
   *
   * The stored response keeps whatever validators the server sent, so this
   * costs one conditional request and usually a 304. Use it for the few large
   * assets whose identity is not otherwise known: the module and the pack.
   */
  async revalidating(url: string): Promise<Response> {
    const key = `${KEY_ORIGIN}url/${encodeURIComponent(url)}`;
    const cached = await this.#read(key);

    const conditional: Record<string, string> = {};
    const etag = cached?.headers.get("etag");
    const modified = cached?.headers.get("last-modified");
    if (etag) conditional["if-none-match"] = etag;
    else if (modified) conditional["if-modified-since"] = modified;

    // A stored copy with no validator can never be shown to be current, and
    // silently running last week's engine is worse than fetching this week's.
    // Every ordinary static host sends one of these; a host that sends neither
    // gets no caching here, which is the honest outcome rather than a stale
    // one.
    const revalidatable = cached && Object.keys(conditional).length > 0;

    let response: Response;
    try {
      response = await fetch(url, { headers: conditional });
    } catch (error) {
      // Offline with a cached copy is exactly the case worth surviving, and a
      // copy that cannot be revalidated is still better than no document.
      if (cached) return cached;
      throw error;
    }

    if (response.status === 304 && revalidatable) return cached;
    if (!response.ok) {
      if (cached) return cached;
      return response;
    }

    await this.#write(key, response.clone());
    return response;
  }

  /** Store an asset the caller already has. */
  async remember(generation: string, name: string, bytes: Uint8Array): Promise<void> {
    await this.#store(`${KEY_ORIGIN}gen/${generation}/${encodeURIComponent(name)}`, bytes);
  }

  /**
   * Every asset stored under a generation, by name.
   *
   * Used to put back what a previous session fetched one blocking read at a
   * time, before anything asks for it again.
   */
  async restore(generation: string): Promise<Map<string, Uint8Array>> {
    const prefix = `${KEY_ORIGIN}gen/${generation}/`;
    const found = new Map<string, Uint8Array>();

    let keys: readonly { url: string }[];
    try {
      keys = await this.#cache.keys();
    } catch {
      return found;
    }

    await Promise.all(
      keys
        .filter((key) => key.url.startsWith(prefix))
        .map(async (key) => {
          const response = await this.#read(key.url);
          if (!response) return;
          const name = decodeURIComponent(key.url.slice(prefix.length));
          found.set(name, new Uint8Array(await response.arrayBuffer()));
        }),
    );

    return found;
  }

  async #read(key: string): Promise<Response | undefined> {
    try {
      return await this.#cache.match(key);
    } catch {
      return undefined;
    }
  }

  async #store(key: string, bytes: Uint8Array): Promise<void> {
    // A copy, because the caller's view may be over WebAssembly memory, which
    // detaches when the heap grows — and a detached buffer stored here would
    // be an empty file on the next load.
    await this.#write(key, new Response(bytes.slice()));
  }

  async #write(key: string, response: Response): Promise<void> {
    try {
      await this.#cache.put(key, response);
    } catch {
      // Over quota, or storage denied. The asset was still fetched; it just
      // will not be here next time.
    }
  }
}

/**
 * A short, stable name for exactly these bytes.
 *
 * Used to turn the bundle URL into a generation.
 */
export async function digest(bytes: Uint8Array): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", bytes.slice());
  return [...new Uint8Array(hash).slice(0, 12)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}
