/** The part of the Cache API this uses. */
export interface CacheLike {
  match(request: string): Promise<Response | undefined>;
  put(request: string, response: Response): Promise<void>;
  keys(): Promise<readonly { url: string }[]>;
}

/** The part of `caches` this uses. */
export interface CacheStorageLike {
  open(name: string): Promise<CacheLike>;
  keys(): Promise<string[]>;
  delete(name: string): Promise<boolean>;
}

const CACHE_PREFIX = "texyon-assets-";
const CACHE_NAME = `${CACHE_PREFIX}v1`;
/** The Cache API keys by URL. This origin is never contacted. */
const KEY_ORIGIN = "https://texyon.invalid/";

/**
 * The engine's assets across page loads. Every failure degrades to a cold
 * fetch, since storage may be absent, denied or full.
 */
export class AssetCache {
  #cache: CacheLike;

  private constructor(cache: CacheLike) {
    this.#cache = cache;
  }

  /** Open the cache and drop older versions, or return null without usable storage. */
  static async open(storage: CacheStorageLike | undefined): Promise<AssetCache | null> {
    if (!storage) return null;
    try {
      const cache = await storage.open(CACHE_NAME);
      const stale = (await storage.keys()).filter(
        (name) => name.startsWith(CACHE_PREFIX) && name !== CACHE_NAME,
      );
      await Promise.all(stale.map((name) => storage.delete(name)));
      return new AssetCache(cache);
    } catch {
      return null;
    }
  }

  /** Fetch `url`, answering from the cache when the server confirms it is current. */
  async revalidating(url: string): Promise<Response> {
    const key = `${KEY_ORIGIN}url/${encodeURIComponent(url)}`;
    const cached = await this.#read(key);

    const conditional: Record<string, string> = {};
    const etag = cached?.headers.get("etag");
    const modified = cached?.headers.get("last-modified");
    if (etag) conditional["if-none-match"] = etag;
    else if (modified) conditional["if-modified-since"] = modified;

    let response: Response;
    try {
      response = await fetch(url, { headers: conditional });
    } catch (error) {
      if (cached) return cached;
      throw error;
    }

    // Without a validator a stored copy can never be shown current, so it is not reused.
    if (response.status === 304 && cached && Object.keys(conditional).length > 0) return cached;
    if (!response.ok) return cached ?? response;

    await this.#write(key, response.clone());
    return response;
  }

  /** Store a file read out of `bundle`, which never changes under its URL. */
  async remember(bundle: string, name: string, bytes: Uint8Array): Promise<void> {
    // Copied, since a view over wasm memory detaches when the heap grows.
    await this.#write(this.#bundlePrefix(bundle) + encodeURIComponent(name), new Response(bytes.slice()));
  }

  /** Every file stored for `bundle`, by name. */
  async restore(bundle: string): Promise<Map<string, Uint8Array>> {
    const prefix = this.#bundlePrefix(bundle);
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
          found.set(decodeURIComponent(key.url.slice(prefix.length)), new Uint8Array(await response.arrayBuffer()));
        }),
    );
    return found;
  }

  #bundlePrefix(bundle: string): string {
    return `${KEY_ORIGIN}bundle/${encodeURIComponent(bundle)}/`;
  }

  async #read(key: string): Promise<Response | undefined> {
    try {
      return await this.#cache.match(key);
    } catch {
      return undefined;
    }
  }

  async #write(key: string, response: Response): Promise<void> {
    try {
      await this.#cache.put(key, response);
    } catch {
      // Over quota or denied. The asset is just not kept.
    }
  }
}
