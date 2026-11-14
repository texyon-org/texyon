import assert from "node:assert/strict";
import { beforeEach, mock, test } from "node:test";

import { AssetCache, digest } from "./cache.ts";
import type { CacheLike, CacheStorageLike } from "./cache.ts";

/** A Cache API standing in for the browser's, with the same aliasing rules. */
class FakeCache implements CacheLike {
  entries = new Map<
    string,
    { body: Uint8Array<ArrayBuffer>; headers: Record<string, string> }
  >();
  /** Set to make every write fail, as a full or disallowed store would. */
  readonly: boolean;

  constructor(readonly_ = false) {
    this.readonly = readonly_;
  }

  async match(request: string): Promise<Response | undefined> {
    const entry = this.entries.get(request);
    if (!entry) return undefined;
    return new Response(entry.body, { headers: entry.headers });
  }

  async put(request: string, response: Response): Promise<void> {
    if (this.readonly) throw new Error("quota exceeded");
    const headers: Record<string, string> = {};
    response.headers.forEach((value, key) => {
      headers[key] = value;
    });
    this.entries.set(request, {
      body: new Uint8Array(await response.arrayBuffer()),
      headers,
    });
  }

  async keys(): Promise<readonly { url: string }[]> {
    return [...this.entries.keys()].map((url) => ({ url }));
  }
}

class FakeStorage implements CacheStorageLike {
  caches = new Map<string, FakeCache>();

  async open(name: string): Promise<CacheLike> {
    let cache = this.caches.get(name);
    if (!cache) {
      cache = new FakeCache();
      this.caches.set(name, cache);
    }
    return cache;
  }

  async keys(): Promise<string[]> {
    return [...this.caches.keys()];
  }

  async delete(name: string): Promise<boolean> {
    return this.caches.delete(name);
  }
}

const GENERATION = "abc123";

beforeEach(() => {
  mock.restoreAll();
});

test("names that differ only by escaping stay separate", async () => {
  // Pack names contain slashes, and a key scheme that did not escape them
  // would let `a/b` and `a%2Fb` collide.
  const storage = new FakeStorage();
  const cache = (await AssetCache.open(storage))!;

  await cache.remember(GENERATION, "chapters/one.tex", new Uint8Array([1]));
  await cache.remember(GENERATION, "chapters%2Fone.tex", new Uint8Array([2]));

  const restored = await cache.restore(GENERATION);
  assert.deepEqual([...restored.get("chapters/one.tex")!], [1]);
  assert.deepEqual([...restored.get("chapters%2Fone.tex")!], [2]);
});

test("restoring a generation returns what it holds, and nothing else", async () => {
  const storage = new FakeStorage();
  const cache = (await AssetCache.open(storage))!;

  await cache.remember("bundle-a", "cmr10.tfm", new Uint8Array([1]));
  await cache.remember("bundle-a", "lmr10.otf", new Uint8Array([2]));
  await cache.remember("bundle-b", "other.tfm", new Uint8Array([3]));

  const restored = await cache.restore("bundle-a");

  assert.deepEqual([...restored.keys()].sort(), ["cmr10.tfm", "lmr10.otf"]);
  assert.deepEqual([...restored.get("cmr10.tfm")!], [1]);
});

test("a rebuilt module is picked up, and an unchanged one is not re-downloaded", async () => {
  const storage = new FakeStorage();
  const cache = (await AssetCache.open(storage))!;

  const calls: (string | undefined)[] = [];
  mock.method(globalThis, "fetch", async (_url: string, init?: RequestInit) => {
    const conditional = new Headers(init?.headers).get("if-none-match");
    calls.push(conditional ?? undefined);

    // The first request has nothing to ask about; the second asks and is told
    // nothing changed; the third is told it did.
    if (calls.length === 1) {
      return new Response(new Uint8Array([1]), { headers: { etag: '"v1"' } });
    }
    if (calls.length === 2) return new Response(null, { status: 304 });
    return new Response(new Uint8Array([9]), { headers: { etag: '"v2"' } });
  });

  const first = await cache.revalidating("https://host.test/engine.wasm");
  assert.deepEqual([...new Uint8Array(await first.arrayBuffer())], [1]);

  const second = await cache.revalidating("https://host.test/engine.wasm");
  assert.equal(calls[1], '"v1"', "the second request should carry the stored validator");
  assert.deepEqual(
    [...new Uint8Array(await second.arrayBuffer())],
    [1],
    "a 304 must be answered from the cache",
  );

  const third = await cache.revalidating("https://host.test/engine.wasm");
  assert.deepEqual(
    [...new Uint8Array(await third.arrayBuffer())],
    [9],
    "a changed module must replace the cached one",
  );
});

test("a module with no validator is fetched again rather than trusted", async () => {
  // Serving a stale engine silently is worse than fetching it twice, and a
  // response with neither an ETag nor a Last-Modified can never be shown to
  // still be current.
  const storage = new FakeStorage();
  const cache = (await AssetCache.open(storage))!;

  let served = 0;
  mock.method(globalThis, "fetch", async () => {
    served++;
    return new Response(new Uint8Array([served]));
  });

  const first = await cache.revalidating("https://host.test/engine.wasm");
  assert.deepEqual([...new Uint8Array(await first.arrayBuffer())], [1]);

  const second = await cache.revalidating("https://host.test/engine.wasm");
  assert.equal(served, 2, "a copy that cannot be revalidated must not be reused");
  assert.deepEqual([...new Uint8Array(await second.arrayBuffer())], [2]);
});

test("a cached module answers when the network is gone", async () => {
  const storage = new FakeStorage();
  const cache = (await AssetCache.open(storage))!;

  let online = true;
  mock.method(globalThis, "fetch", async () => {
    if (!online) throw new TypeError("Failed to fetch");
    return new Response(new Uint8Array([7]), { headers: { etag: '"v1"' } });
  });

  await cache.revalidating("https://host.test/engine.wasm");
  online = false;
  const offline = await cache.revalidating("https://host.test/engine.wasm");

  assert.deepEqual([...new Uint8Array(await offline.arrayBuffer())], [7]);
});

test("storage that refuses to write still serves the document", async () => {
  // A private window, or an origin over quota. The asset is fetched; it simply
  // does not persist.
  const storage = new FakeStorage();
  storage.caches.set("tectonic-assets-v2", new FakeCache(true));
  const cache = (await AssetCache.open(storage))!;

  await cache.remember(GENERATION, "cmr10.tfm", new Uint8Array([5]));

  assert.equal((await cache.restore(GENERATION)).size, 0);
});

test("no storage at all is not an error", async () => {
  assert.equal(await AssetCache.open(undefined), null);

  const broken: CacheStorageLike = {
    open: async () => {
      throw new Error("denied");
    },
    keys: async () => [],
    delete: async () => false,
  };
  assert.equal(await AssetCache.open(broken), null);
});

test("caches from an older version of this code are cleaned up", async () => {
  const storage = new FakeStorage();
  storage.caches.set("tectonic-assets-v1", new FakeCache());
  storage.caches.set("something-else", new FakeCache());

  await AssetCache.open(storage);

  assert.deepEqual((await storage.keys()).sort(), [
    "something-else",
    "tectonic-assets-v2",
  ]);
});

test("a digest changes when the bytes do", async () => {
  const one = await digest(new TextEncoder().encode('{"files":{"a":1}}'));
  const same = await digest(new TextEncoder().encode('{"files":{"a":1}}'));
  const other = await digest(new TextEncoder().encode('{"files":{"a":2}}'));

  assert.equal(one, same);
  assert.notEqual(one, other);
  assert.match(one, /^[0-9a-f]{24}$/);
});
