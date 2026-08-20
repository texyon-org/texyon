/// <reference lib="webworker" />

/**
 * The Worker that owns the engine.
 *
 * Everything about this file follows from one constraint: TeX asks for files in
 * the middle of typesetting and cannot yield while it waits. That is why the
 * engine lives in a Worker at all — a Worker is the one place a browser still
 * lets you block, so `tectonic_host_read_file` can be answered with a
 * synchronous `XMLHttpRequest` without freezing the page.
 *
 * Blocking is the fallback, not the plan. The support pack is pulled into
 * memory in the background as soon as the engine is up, so in steady state
 * every file request is a `Map` lookup and the network is nowhere near the
 * keystroke path.
 */

import type {
  CompileResult,
  Diagnostic,
  EngineCommand,
  PageOverlay,
  LogLevel,
  SerializableEngineOptions,
  SourceSpot,
  SyncRect,
  WorkerRequest,
  WorkerResponse,
} from "./protocol.js";
// Relative imports carry their extension because this file is emitted as an ES
// module and loaded by the browser directly — the engine starts it with
// `new URL("./worker.js", import.meta.url)`, so no bundler is in the way to
// resolve a bare "./cache". Without the extension the worker fails to load, and
// a module worker that fails to load reports nothing at all.
import { gunzipSync } from "fflate";

import { AssetCache, digest } from "./cache.js";
import { parseTtbHeader, TtbIndex, TTB_HEADER_BYTES } from "./ttb.js";
import type { TtbLocation } from "./ttb.js";
import { Wasi, WasiExit } from "./wasi.js";

/** The engine's exported ABI. See `crates/wasm/src/lib.rs`. */
interface Exports {
  memory: WebAssembly.Memory;
  _initialize?: () => void;
  tectonic_request(
    reqPtr: number,
    reqLen: number,
    outPtr: number,
    outLen: number,
  ): number;
  tectonic_wasm_alloc(len: number): number;
  tectonic_wasm_free(ptr: number, len: number): void;
}

/** What the Rust side reports for one compile. */
interface RawCompileReport {
  /** The SVG itself stays in linear memory; see `collect`. */
  pages: { index: number; ptr: number; len: number }[];
  changed: number[];
  removed: number[];
  pageSizes: [number, number][];
  pageOverlays: PageOverlay[];
  pageCount: number;
  log: string;
  outcome: CompileResult["outcome"];
  aborted?: string;
  tailPending: boolean;
  messages: string[];
  diagnostics: Diagnostic[];
  checkpointRungs: number;
  checkpointBytes: number;
  reusedPrefix: boolean;
  shipouts: number;
  pagesFingerprinted: number;
  pagesRendered: number;
  unsupportedPages: number;
  replayRung: number;
  replayCommands: number;
  texMs: number;
  analyzeMs: number;
  renderMs: number;
  supportFiles: number;
  supportBytes: number;
}

/**
 * What the Rust side replies with.
 *
 * The enum is internally tagged, so a compile report arrives with its fields
 * alongside `status` rather than nested under it.
 */
type RawResponse =
  | { status: "ok" }
  | { status: "located"; rect: SyncRect | null; source: SourceSpot | null }
  | { status: "commands"; commands: EngineCommand[] }
  | ({ status: "compiled" } & RawCompileReport)
  | { status: "exported"; ptr: number; len: number }
  | { status: "idle" }
  | { status: "error"; message: string };

const encoder = new TextEncoder();
const decoder = new TextDecoder();

let exports: Exports | null = null;
let options: SerializableEngineOptions | null = null;

/** Support files already in memory, by TeX name. */
const resident = new Map<string, Uint8Array>();
/** Project binary files by path, consulted before the support pack. */
const projectResources = new Map<string, Uint8Array>();
/**
 * Project text files by path, with a revision that moves when the text does.
 *
 * The compile request into the module carries only `{path: revision}`; the
 * session pulls the text of any revision it does not hold back out through
 * `tectonic_host_read_source`. A reopened session holds nothing and simply
 * pulls everything, which is why no resend bookkeeping exists anywhere.
 */
const projectSources = new Map<string, { text: string; revision: number }>();
let nextSourceRevision = 1;
/** Engine caches and checkpoints assume immutable files, a binary change forces a reopen. */
let restartPending = false;
let compiledSinceOpen = false;
/** Kept so a restart can reopen the session. */
let formatName = "";
/** Names the pack claims to have, with their sizes. */
let manifest = new Map<string, number>();
/** Names known not to exist, so a miss is not re-fetched on every keystroke. */
const missing = new Set<string>();
/** null until the bundle index arrives, then how that ended. */
let indexOutcome: "ready" | "failed" | null = null;
/** A lookup failed only because the index was not there yet. */
let missedAwaitingIndex = false;
/**
 * Where each file lives in the bundle tar, once its index has arrived.
 *
 * The pack is a fixed head start; this is the long tail — every package, font
 * and graphic in TeX Live, addressed by a byte range.
 */
let bundle = new Map<string, [offset: number, length: number]>();
/**
 * The same long tail, when the bundle is a `.ttb` rather than a tar.
 *
 * A ttb brings its own index and search rules, so names are resolved through
 * it instead of a flat map — and its blobs are gzipped, so a read is a range
 * request plus an inflate rather than a plain copy.
 */
let ttb: TtbIndex | null = null;

/** Nanosecond-ish timing for the stats the UI shows. */
let engineStart = 0;

/** Assets held across page loads, or null where storage is unavailable. */
let assets: AssetCache | null = null;
/** Identifies the pack build, so a rebuild is not answered from the old one. */
let packGeneration = "";
/** Identifies the bundle, which is immutable under its own URL. */
let bundleGeneration = "";
/** Bundle files fetched this session that are not yet persisted. */
const unsaved = new Map<string, Uint8Array>();

/**
 * Persist what the blocking path fetched.
 *
 * Bundle reads happen synchronously from inside the typesetting loop, where
 * nothing asynchronous can be awaited, so they are queued here and written out
 * once the engine hands control back.
 */
async function flushUnsaved(): Promise<void> {
  if (!assets || !bundleGeneration || unsaved.size === 0) return;

  const pending = [...unsaved];
  unsaved.clear();
  for (const [name, bytes] of pending) {
    await assets.remember(bundleGeneration, name, bytes);
  }
}

function post(message: WorkerResponse, transfer: Transferable[] = []): void {
  (self as DedicatedWorkerGlobalScope).postMessage(message, transfer);
}

function fileUrl(name: string): string {
  // supportUrl is already absolute — the page resolved it — and is guaranteed
  // to end in a slash by the caller's convention.
  return new URL(`files/${encodeURIComponent(name)}`, options!.supportUrl).href;
}

/**
 * Fetch one support file, blocking the Worker until it arrives.
 *
 * Only reached for a file the background prefetch has not delivered yet.
 */
function fetchSync(url: string, range?: [number, number]): Uint8Array | null {
  const request = new XMLHttpRequest();
  request.open("GET", url, /* async */ false);
  request.responseType = "arraybuffer";

  if (range) {
    const [offset, length] = range;
    request.setRequestHeader("Range", `bytes=${offset}-${offset + length - 1}`);
  }

  try {
    request.send();
  } catch {
    return null;
  }

  // 206 for a range, 200 for a whole file — or for a server that ignored the
  // Range header, which would hand us 2.8 GB, so insist on 206 when we asked.
  if (range ? request.status !== 206 : request.status !== 200) return null;

  // With a synchronous XHR the browser hands back the buffer directly on some
  // engines and a string on others; responseType is honoured either way in a
  // Worker, but guard anyway.
  const body = request.response as ArrayBuffer | null;
  return body ? new Uint8Array(body) : null;
}

/**
 * Check that what came back is really the file we asked for.
 *
 * A single-page-app host answers an unknown path with `index.html` and HTTP
 * 200, not a 404 — so without this the engine is handed a web page and told it
 * is `pdftex.map`, which it dutifully parses into nonsense. The manifest knows
 * each file's exact length, which makes the check both cheap and precise.
 */
function verify(name: string, bytes: Uint8Array): Uint8Array | null {
  const expected = manifest.get(name);
  if (expected === undefined || bytes.length === expected) return bytes;

  post({
    id: 0,
    kind: "log",
    line: {
      level: "error",
      message:
        `the support file ${name} came back as ${bytes.length} bytes where the ` +
        `manifest says ${expected} — the server is probably answering unknown ` +
        `paths with a fallback page rather than a 404`,
    },
  });

  return null;
}

/** The engine may spell a project path as /doc/x or ./x. */
function projectResource(name: string): Uint8Array | null {
  let path = name;
  if (path.startsWith("/doc/")) path = path.slice("/doc/".length);
  while (path.startsWith("./")) path = path.slice(2);
  return projectResources.get(path) ?? null;
}

/** Answer the engine's request for a support file. */
function readFile(name: string): Uint8Array | null {
  const fromProject = projectResource(name);
  if (fromProject) return fromProject;

  const hit = resident.get(name);
  if (hit) return hit;
  if (missing.has(name)) return null;

  const fromThePack = fromPack(name);
  if (fromThePack) {
    resident.set(name, fromThePack);
    return fromThePack;
  }

  const at = bundle.get(name);
  const inTtb = at ? null : (ttb?.resolve(name) ?? null);

  if (!at && !inTtb) {
    // Genuinely absent. TeX probes many candidate names per lookup, so
    // remembering that is what keeps a lookup from being a round trip each
    // time. A miss before the index has arrived is provisional and asks for a
    // recompile once it has.
    if (options?.bundleUrl !== undefined && indexOutcome === null) missedAwaitingIndex = true;
    missing.add(name);
    return null;
  }

  const bytes = at ? fromBundle(name, at) : fromTtb(name, inTtb!);
  if (bytes) {
    resident.set(name, bytes);
    return bytes;
  }

  // The file exists but we could not read it. Deliberately *not* remembered as
  // missing: a transient failure would otherwise turn into a package that is
  // permanently unavailable for the rest of the session, and the next keystroke
  // is a perfectly good moment to try again.
  return null;
}

/** Look in the local support pack. */
function fromPack(name: string): Uint8Array | null {
  // The manifest is authoritative about what the pack holds, so a name that is
  // not in it can be skipped without a request.
  if (manifest.size > 0 && !manifest.has(name)) return null;

  const bytes = fetchSync(fileUrl(name));
  return bytes ? verify(name, bytes) : null;
}

/** Read one file out of the bundle tar, one range request. */
function fromBundle(name: string, [offset, length]: [number, number]): Uint8Array | null {
  const url = options?.bundleUrl;
  if (!url) return null;

  // A cold package pulls hundreds of these in a row and the occasional one does
  // fail; a couple of retries is much cheaper than surfacing that to the user
  // as a missing file.
  for (let attempt = 0; attempt < 3; attempt++) {
    const bytes = fetchSync(url, [offset, length]);
    if (bytes && bytes.length === length) {
      // A bundle range request is the most expensive read there is, and the
      // same document asks for the same files on every cold start.
      unsaved.set(name, bytes);
      return bytes;
    }
  }

  post({
    id: 0,
    kind: "log",
    line: {
      level: "error",
      message:
        `could not read ${name} from the bundle after 3 attempts ` +
        `(${length} bytes at ${offset}) — it will be retried`,
    },
  });

  return null;
}

/**
 * Read one file out of a ttb: one range request, then an inflate.
 *
 * The inflate has to be synchronous — this runs inside the typesetting loop,
 * where nothing can be awaited — which is what fflate is here for. What goes
 * into the cache is the decompressed bytes, so a warm start needs neither the
 * request nor the inflate.
 */
function fromTtb(name: string, { offset, gzLength, realLength }: TtbLocation): Uint8Array | null {
  const url = options?.bundleUrl;
  if (!url) return null;

  for (let attempt = 0; attempt < 3; attempt++) {
    const compressed = fetchSync(url, [offset, gzLength]);
    if (!compressed || compressed.length !== gzLength) continue;

    try {
      const bytes = gunzipSync(compressed);
      // The index promised this length; anything else means the range and the
      // index disagree, and bad bytes are worse than a missing file.
      if (bytes.length === realLength) {
        unsaved.set(name, bytes);
        return bytes;
      }
    } catch {
      // Damaged in transit; the next attempt refetches.
    }
  }

  post({
    id: 0,
    kind: "log",
    line: {
      level: "error",
      message:
        `could not read ${name} from the bundle after 3 attempts ` +
        `(${gzLength} bytes at ${offset}) — it will be retried`,
    },
  });

  return null;
}

/**
 * Fetch an exact byte range, insisting the server honoured it.
 *
 * A 200 means the Range header was ignored and the whole multi-gigabyte
 * bundle is on its way; cancelling and failing is the only sane response.
 */
async function fetchRange(url: string, offset: number, length: number): Promise<Uint8Array> {
  const response = await fetch(url, {
    headers: { Range: `bytes=${offset}-${offset + length - 1}` },
  });

  if (response.status !== 206) {
    await response.body?.cancel();
    throw new Error(`expected 206 for a range request, got HTTP ${response.status}`);
  }

  return new Uint8Array(await response.arrayBuffer());
}

/**
 * Fetch and parse the bundle index.
 *
 * Two dialects. A tar bundle keeps a gzipped sidecar of `name offset length`
 * lines next to itself; a `.ttb` carries its index inside, at an offset its
 * header states. Either way this is pulled in the background so that no
 * compile ever waits for it; until it lands, the pack answers on its own.
 */
async function indexBundle(url: string): Promise<void> {
  post({ id: 0, kind: "progress", progress: { phase: "indexing-bundle" } });

  const isTtb = url.endsWith(".ttb");

  try {
    let count: number;

    if (isTtb) {
      const header = parseTtbHeader(await fetchRange(url, 0, TTB_HEADER_BYTES));
      const compressed = await fetchRange(url, header.indexOffset, header.indexGzLength);
      ttb = TtbIndex.parse(new TextDecoder().decode(gunzipSync(compressed)));
      count = ttb.size;
    } else {
      const response = await fetch(`${url}.index.gz`);
      if (!response.ok || !response.body) {
        throw new Error(`HTTP ${response.status}`);
      }

      // The file is gzip *content*, not a gzip content-encoding, so the browser
      // hands it over compressed and we undo that ourselves.
      const stream = response.body.pipeThrough(new DecompressionStream("gzip"));
      const text = await new Response(stream).text();

      const parsed = new Map<string, [number, number]>();

      for (const line of text.split("\n")) {
        if (!line) continue;
        const first = line.indexOf(" ");
        const second = line.indexOf(" ", first + 1);
        if (first < 0 || second < 0) continue;

        parsed.set(line.slice(0, first), [
          Number(line.slice(first + 1, second)),
          Number(line.slice(second + 1)),
        ]);
      }

      bundle = parsed;
      count = bundle.size;
    }

    // Names ruled out before the index arrived deserve another chance. The
    // session cached those misses too and diffs sources by content, so only
    // a reopen makes a recompile actually look again.
    indexOutcome = "ready";
    missing.clear();
    if (missedAwaitingIndex) {
      missedAwaitingIndex = false;
      if (compiledSinceOpen) restartPending = true;
      post({ id: 0, kind: "recompile" });
    }

    post({
      id: 0,
      kind: "log",
      line: {
        level: "note",
        message: `bundle ready: ${count.toLocaleString()} files available on demand`,
      },
    });
  } catch (error) {
    indexOutcome = "failed";
    post({
      id: 0,
      kind: "log",
      line: {
        level: "error",
        message:
          `could not load the bundle index from ${isTtb ? url : `${url}.index.gz`} ` +
          `(${error instanceof Error ? error.message : String(error)}) — ` +
          "only files in the local pack will be available",
      },
    });
  }

  post({ id: 0, kind: "progress", progress: { phase: "ready" } });
}

/**
 * One support file, from the cache when this build has already fetched it.
 *
 * Verified against the manifest either way: a cached response is only as
 * trustworthy as what was stored, and a single-page-app host that answered a
 * missing file with its index page would otherwise persist that mistake.
 */
async function packFile(name: string): Promise<Uint8Array | null> {
  const download = async (): Promise<Uint8Array | null> => {
    const response = await fetch(fileUrl(name));
    if (!response.ok) return null;
    return verify(name, new Uint8Array(await response.arrayBuffer()));
  };

  if (!assets || !packGeneration) return download();

  const bytes = await assets.generational(packGeneration, name, download);
  return bytes ? verify(name, bytes) : null;
}

/** Load the whole pack in the background so the blocking path stops firing. */
async function prefetch(): Promise<void> {
  const names = [...manifest.keys()].filter((name) => !resident.has(name));
  let loaded = 0;
  const total = names.length;

  post({ id: 0, kind: "progress", progress: { phase: "prefetching", loaded, total } });

  // Enough concurrency to saturate a connection without opening 350 sockets.
  const width = 8;
  let next = 0;

  const worker = async (): Promise<void> => {
    for (;;) {
      const index = next++;
      if (index >= names.length) return;
      const name = names[index];

      try {
        const bytes = await packFile(name);
        // A synchronous read may have landed the same file meanwhile; first
        // one wins, they are identical.
        if (bytes && !resident.has(name)) resident.set(name, bytes);
      } catch {
        // A file that fails to prefetch is simply left to the blocking path.
      }

      loaded++;
      if (loaded % 25 === 0 || loaded === total) {
        post({
          id: 0,
          kind: "progress",
          progress: { phase: "prefetching", loaded, total },
        });
      }
    }
  };

  await Promise.all(Array.from({ length: width }, worker));
  post({ id: 0, kind: "progress", progress: { phase: "ready" } });
}

/** Copy a JS byte array into the module and return its address. */
function intoModule(bytes: Uint8Array): { ptr: number; len: number } {
  const ptr = exports!.tectonic_wasm_alloc(bytes.length);
  if (ptr === 0 && bytes.length > 0) throw new Error("the engine is out of memory");
  new Uint8Array(exports!.memory.buffer, ptr, bytes.length).set(bytes);
  return { ptr, len: bytes.length };
}

/** Send one JSON request into the module and decode its reply. */
function call(request: unknown): RawResponse {
  const engine = exports!;
  const encoded = encoder.encode(JSON.stringify(request));
  const input = intoModule(encoded);

  // Two words of scratch for the reply's (pointer, length).
  const out = engine.tectonic_wasm_alloc(8);

  try {
    const rc = engine.tectonic_request(input.ptr, input.len, out, out + 4);
    if (rc !== 0) throw new Error("the engine could not produce a response");

    const view = new DataView(engine.memory.buffer);
    const replyPtr = view.getUint32(out, true);
    const replyLen = view.getUint32(out + 4, true);
    const reply = decoder.decode(
      new Uint8Array(engine.memory.buffer, replyPtr, replyLen).slice(),
    );
    engine.tectonic_wasm_free(replyPtr, replyLen);

    return JSON.parse(reply) as RawResponse;
  } finally {
    engine.tectonic_wasm_free(out, 8);
    engine.tectonic_wasm_free(input.ptr, input.len);
  }
}

/** Bring up the module and open a session. */
async function init(opts: SerializableEngineOptions): Promise<void> {
  options = opts;

  // Absent in a runtime without the Cache API, and refused in some private
  // windows. Either way the engine still starts; it just starts cold.
  assets = await AssetCache.open(typeof caches === "undefined" ? undefined : caches);
  bundleGeneration = opts.bundleUrl ? await digest(encoder.encode(opts.bundleUrl)) : "";

  post({ id: 0, kind: "progress", progress: { phase: "fetching-module" } });

  const wasi = new Wasi({
    onOutput: (stream, text) => {
      post({
        id: 0,
        kind: "log",
        line: { level: stream === "stderr" ? "error" : "note", message: text },
      });
    },
    // TeX bakes the date into `\today` and into the PDF; pinning it keeps a
    // recompile of unchanged text byte-identical, which is what lets the
    // preview skip work.
    now: () => 0,
  });

  const imports: WebAssembly.Imports = {
    wasi_snapshot_preview1: wasi.imports as unknown as WebAssembly.ModuleImports,
    env: {
      tectonic_host_read_file(
        namePtr: number,
        nameLen: number,
        outPtr: number,
        outLen: number,
      ): number {
        const engine = exports!;
        const name = decoder.decode(
          new Uint8Array(engine.memory.buffer, namePtr, nameLen),
        );

        const bytes = readFile(name);
        if (!bytes) return 1; // not found

        const buffer = intoModule(bytes);
        const view = new DataView(engine.memory.buffer);
        view.setUint32(outPtr, buffer.ptr, true);
        view.setUint32(outLen, buffer.len, true);
        return 0;
      },

      tectonic_host_read_source(
        namePtr: number,
        nameLen: number,
        outPtr: number,
        outLen: number,
      ): number {
        const engine = exports!;
        const name = decoder.decode(
          new Uint8Array(engine.memory.buffer, namePtr, nameLen),
        );

        const source = projectSources.get(name);
        if (source === undefined) return 1; // not found

        const buffer = intoModule(encoder.encode(source.text));
        const view = new DataView(engine.memory.buffer);
        view.setUint32(outPtr, buffer.ptr, true);
        view.setUint32(outLen, buffer.len, true);
        return 0;
      },

      tectonic_host_log(kind: number, msgPtr: number, msgLen: number): void {
        const engine = exports!;
        const message = decoder.decode(
          new Uint8Array(engine.memory.buffer, msgPtr, msgLen),
        );
        const level: LogLevel = kind === 2 ? "error" : kind === 1 ? "warning" : "note";
        post({ id: 0, kind: "log", line: { level, message } });
      },
    },
  };

  const source = assets
    ? await assets.revalidating(opts.moduleUrl)
    : await fetch(opts.moduleUrl);
  if (!source.ok) {
    throw new Error(`could not fetch ${opts.moduleUrl}: HTTP ${source.status}`);
  }

  post({ id: 0, kind: "progress", progress: { phase: "instantiating" } });

  const { instance } = await WebAssembly.instantiateStreaming(source, imports).catch(
    async () => {
      // instantiateStreaming needs application/wasm; fall back for dev servers
      // and file:// origins that do not set it.
      const bytes = await (await fetch(opts.moduleUrl)).arrayBuffer();
      return WebAssembly.instantiate(bytes, imports);
    },
  );

  exports = instance.exports as unknown as Exports;
  wasi.bind(exports.memory);
  // A cdylib is a reactor: no `_start`, but libc still wants initializing.
  exports._initialize?.();

  // The manifest tells us the pack's contents, which lets a miss be answered
  // without a request.
  const manifestResponse = await fetch(`${opts.supportUrl}manifest.json`);
  if (!manifestResponse.ok) {
    throw new Error(
      `could not fetch the support manifest at ${opts.supportUrl}manifest.json — ` +
        "run tools/wasm/build-assets.sh",
    );
  }
  // Read as bytes first: the manifest names every file in the pack and how long
  // it is, so its digest is an identity for the whole pack. That turns several
  // hundred cache validations into this one small request, which had to happen
  // anyway.
  const manifestBytes = new Uint8Array(await manifestResponse.arrayBuffer());
  packGeneration = assets ? await digest(manifestBytes) : "";
  const parsed = JSON.parse(decoder.decode(manifestBytes)) as {
    format: string;
    files: Record<string, number>;
  };
  manifest = new Map(Object.entries(parsed.files));

  // Put back what earlier sessions pulled out of the bundle, before the
  // blocking path can ask for any of it again.
  if (assets && bundleGeneration) {
    for (const [name, bytes] of await assets.restore(bundleGeneration)) {
      if (!resident.has(name)) resident.set(name, bytes);
    }
  }

  // The format dump is 24 MB and nothing can start without it, so it is fetched
  // up front rather than through the blocking path.
  post({
    id: 0,
    kind: "progress",
    progress: { phase: "fetching-format", total: manifest.get(parsed.format) },
  });

  const formatBytes = await packFile(parsed.format);
  if (!formatBytes) {
    throw new Error(`could not fetch the format dump ${parsed.format}`);
  }
  resident.set(parsed.format, formatBytes);

  formatName = parsed.format;
  const opened = call({ op: "open", format: parsed.format, primary: opts.primary });
  if (opened.status === "error") throw new Error(opened.message);

  post({ id: 0, kind: "progress", progress: { phase: "ready" } });

  if (opts.prefetch) {
    // Deliberately not awaited: the first compile can start now and take the
    // blocking path for anything the prefetch has not reached yet.
    void prefetch();
  }

  if (opts.bundleUrl) {
    // Likewise: the pack covers the common case, and the index only has to be
    // there by the time someone reaches for a package it does not hold.
    void indexBundle(opts.bundleUrl);
  }
}

/** Turn the module's compile report into the public result. */
function collect(report: RawCompileReport, elapsedMs: number): CompileResult {
  const engine = exports!;

  // The SVG travels as raw bytes in linear memory rather than inside the JSON,
  // like the PDF: decode each page out and give the module its bytes back.
  const pages = report.pages.map(({ index, ptr, len }) => {
    const svg = len > 0 ? decoder.decode(new Uint8Array(engine.memory.buffer, ptr, len)) : "";
    engine.tectonic_wasm_free(ptr, len);
    return { index, svg };
  });

  return {
    pages,
    changed: report.changed,
    removed: report.removed,
    pageSizes: report.pageSizes,
    pageOverlays: report.pageOverlays,
    pageCount: report.pageCount,
    log: report.log,
    outcome: report.outcome,
    aborted: report.aborted,
    tailPending: report.tailPending,
    messages: report.messages,
    diagnostics: report.diagnostics,
    stats: {
      elapsedMs,
      texMs: report.texMs,
      analyzeMs: report.analyzeMs,
      renderMs: report.renderMs,
      checkpointRungs: report.checkpointRungs,
      checkpointBytes: report.checkpointBytes,
      replayRung: report.replayRung,
      replayCommands: report.replayCommands,
      reusedPrefix: report.reusedPrefix,
      shipouts: report.shipouts,
      pagesFingerprinted: report.pagesFingerprinted,
      pagesRendered: report.pagesRendered,
      unsupportedPages: report.unsupportedPages,
      supportFiles: report.supportFiles,
      supportBytes: report.supportBytes,
    },
  };
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;

  try {
    switch (request.kind) {
      case "init":
        await init(request.options);
        post({ id: request.id, kind: "ok" });
        break;

      case "resources": {
        for (const [path, bytes] of Object.entries(request.upserts)) {
          projectResources.set(path, bytes);
        }
        for (const path of request.removed) projectResources.delete(path);

        // a previous miss may exist now
        missing.clear();
        if (compiledSinceOpen) restartPending = true;

        post({ id: request.id, kind: "ok" });
        break;
      }

      case "compile":
      case "continueTail": {
        if (restartPending && request.kind === "compile") {
          // caches and checkpoints predate the new bytes
          call({ op: "shutdown" });
          const reopened = call({ op: "open", format: formatName, primary: options!.primary });
          if (reopened.status === "error") throw new Error(reopened.message);
          restartPending = false;
          compiledSinceOpen = false;
        }

        if (request.kind === "compile") {
          for (const [path, text] of Object.entries(request.upserts)) {
            const held = projectSources.get(path);
            // The revision moves only when the text does, so a re-sent
            // identical file does not read as an edit.
            if (held === undefined || held.text !== text) {
              projectSources.set(path, { text, revision: nextSourceRevision++ });
            }
          }
          for (const path of request.removed) projectSources.delete(path);
        }

        engineStart = performance.now();
        let reply: RawResponse;
        if (request.kind === "compile") {
          const files: Record<string, number> = {};
          for (const [path, { revision }] of projectSources) files[path] = revision;
          reply = call({ op: "compile", files, viewport: request.viewport });
        } else {
          reply = call({ op: "continueTail" });
        }
        const elapsed = performance.now() - engineStart;

        if (reply.status === "error") {
          post({ id: request.id, kind: "error", message: reply.message });
        } else if (reply.status === "idle") {
          post({ id: request.id, kind: "idle" });
        } else if (reply.status === "compiled") {
          compiledSinceOpen = true;
          const result = collect(reply, elapsed);
          post({ id: request.id, kind: "compiled", result });
          // Deliberately not awaited: the frame is already on its way, and
          // persisting what this compile pulled from the bundle can happen
          // while the user reads it.
          void flushUnsaved();
        } else {
          post({ id: request.id, kind: "ok" });
        }
        break;
      }

      case "export": {
        const started = performance.now();
        const reply = call({ op: "export" });

        if (reply.status === "error") {
          post({ id: request.id, kind: "error", message: reply.message });
        } else if (reply.status === "exported") {
          const engine = exports!;
          // The PDF is in linear memory rather than in the JSON. Copy it out
          // before anything else can allocate — a growing heap detaches the
          // buffer this view is over — then give the module its bytes back.
          const pdf = new Uint8Array(engine.memory.buffer, reply.ptr, reply.len).slice();
          engine.tectonic_wasm_free(reply.ptr, reply.len);

          // Transferred, not copied, so a fifty-page document does not cross
          // the Worker boundary twice.
          post(
            {
              id: request.id,
              kind: "exported",
              pdf,
              elapsedMs: performance.now() - started,
            },
            [pdf.buffer],
          );
        } else {
          post({ id: request.id, kind: "error", message: "the engine did not return a PDF" });
        }
        break;
      }

      case "commands": {
        const reply = call({ op: "commands" });
        if (reply.status === "error") {
          post({ id: request.id, kind: "error", message: reply.message });
        } else if (reply.status === "commands") {
          post({ id: request.id, kind: "commands", commands: reply.commands });
        } else {
          post({ id: request.id, kind: "error", message: "the engine did not list its commands" });
        }
        break;
      }

      case "forwardSearch":
      case "backwardSearch": {
        const reply =
          request.kind === "forwardSearch"
            ? call({ op: "forwardSearch", path: request.path, line: request.line })
            : call({
                op: "backwardSearch",
                pageIndex: request.pageIndex,
                hBp: request.hBp,
                vBp: request.vBp,
              });

        if (reply.status === "error") {
          post({ id: request.id, kind: "error", message: reply.message });
        } else if (reply.status === "located") {
          post({ id: request.id, kind: "located", rect: reply.rect, source: reply.source });
        } else {
          post({ id: request.id, kind: "error", message: "the engine did not answer the query" });
        }
        break;
      }

      case "cancel":
        call({ op: "cancel" });
        post({ id: request.id, kind: "ok" });
        break;

      case "shutdown":
        call({ op: "shutdown" });
        post({ id: request.id, kind: "ok" });
        break;
    }
  } catch (error) {
    const message =
      error instanceof WasiExit
        ? `${error.message} — the engine must be restarted`
        : error instanceof Error
          ? error.message
          : String(error);
    post({ id: request.id, kind: "error", message });
  }
};
