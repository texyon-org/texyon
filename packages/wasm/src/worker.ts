/// <reference lib="webworker" />

import { gunzipSync } from "fflate";

import { AssetCache } from "./cache.js";
import { type Buffer, Engine } from "./engine.js";
import type {
  CompileResult,
  Diagnostic,
  EngineCommand,
  LogLevel,
  LogLine,
  PageOverlay,
  PageRaster,
  Reply,
  SourceSpot,
  SyncRect,
  WorkerOptions,
  WorkerRequest,
  WorkerResponse,
} from "./protocol.js";
import { applyEdit } from "./splice.js";
import { inflate, untar } from "./tar.js";
import { parseTtbHeader, TTB_HEADER_BYTES, TtbIndex, type TtbLocation } from "./ttb.js";
import { WasiExit } from "./wasi.js";

/** A page drawn to pixels: width and height as little-endian u32, then RGBA rows. */
interface RawRaster extends Buffer {
  index: number;
}

interface RawCompileReport {
  published: boolean;
  pages: (Buffer & { index: number })[];
  rasters: RawRaster[];
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

type RawResponse =
  | { status: "ok" }
  | { status: "idle" }
  | { status: "error"; message: string }
  | ({ status: "compiled" } & RawCompileReport)
  | ({ status: "passed"; rerun: boolean } & RawCompileReport)
  | ({ status: "exported" } & Buffer)
  | { status: "rastered"; pages: RawRaster[] }
  | { status: "located"; rect: SyncRect | null; boxes?: SyncRect[]; source: SourceSpot | null }
  | { status: "commands"; commands: EngineCommand[] };

const ATTEMPTS = 3;

const decoder = new TextDecoder();

let engine: Engine | null = null;
let options: WorkerOptions | null = null;
let format = "";
let assets: AssetCache | null = null;

/** Support files in memory, by TeX name. */
const resident = new Map<string, Uint8Array>();
/** Names known to be absent, since TeX probes many candidates per lookup. */
const missing = new Set<string>();
/** Bundle files fetched since the last flush to the cache. */
const unsaved = new Map<string, Uint8Array>();
const projectResources = new Map<string, Uint8Array>();
const projectSources = new Map<string, { text: string; revision: number }>();

let bundle: TtbIndex | null = null;
let bundleState: "none" | "pending" | "ready" | "failed" = "none";
let missedAwaitingIndex = false;
/** Engine caches and checkpoints assume immutable files, so a binary change reopens the session. */
let restartPending = false;
let compiledSinceOpen = false;
let rasterScale: number | undefined;

function wasm(): Engine {
  if (!engine) throw new Error("the engine is not running");
  return engine;
}

function post(message: WorkerResponse, transfer: Transferable[] = []): void {
  (self as DedicatedWorkerGlobalScope).postMessage(message, transfer);
}

function log(level: LogLevel, message: string): void {
  post({ kind: "log", line: { level, message } satisfies LogLine });
}

/** Blocking range request, since TeX cannot yield mid-compile. */
function fetchRangeSync(url: string, { offset, gzLength }: TtbLocation): Uint8Array | null {
  const request = new XMLHttpRequest();
  request.open("GET", url, false);
  request.responseType = "arraybuffer";
  request.setRequestHeader("Range", `bytes=${offset}-${offset + gzLength - 1}`);
  try {
    request.send();
  } catch {
    return null;
  }
  // A 200 means the Range header was ignored and the whole bundle is coming.
  if (request.status !== 206) return null;
  const body = request.response as ArrayBuffer | null;
  return body ? new Uint8Array(body) : null;
}

async function fetchRange(url: string, offset: number, length: number): Promise<Uint8Array> {
  const response = await fetch(url, { headers: { Range: `bytes=${offset}-${offset + length - 1}` } });
  if (response.status !== 206) {
    await response.body?.cancel();
    throw new Error(`expected 206 for a range request, got HTTP ${response.status}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

function projectResource(name: string): Uint8Array | undefined {
  let path = name.startsWith("/doc/") ? name.slice("/doc/".length) : name;
  while (path.startsWith("./")) path = path.slice(2);
  return projectResources.get(path);
}

function readFile(name: string): Uint8Array | null {
  const hit = projectResource(name) ?? resident.get(name);
  if (hit) return hit;
  if (missing.has(name)) return null;

  const at = bundle?.resolve(name);
  if (!at) {
    if (bundleState === "pending") missedAwaitingIndex = true;
    missing.add(name);
    return null;
  }

  const bytes = fromBundle(name, at);
  // A failed read is not remembered as missing, so the next compile retries it.
  if (bytes) resident.set(name, bytes);
  return bytes;
}

function fromBundle(name: string, at: TtbLocation): Uint8Array | null {
  const url = options?.bundleUrl;
  if (!url) return null;
  for (let attempt = 0; attempt < ATTEMPTS; attempt++) {
    const compressed = fetchRangeSync(url, at);
    if (compressed?.length !== at.gzLength) continue;
    try {
      const bytes = gunzipSync(compressed);
      if (bytes.length === at.realLength) {
        unsaved.set(name, bytes);
        return bytes;
      }
    } catch {
      // Damaged in transit, refetched on the next attempt.
    }
  }
  log("error", `could not read ${name} from the bundle, it will be retried`);
  return null;
}

async function indexBundle(url: string): Promise<void> {
  post({ kind: "progress", progress: { phase: "indexing-bundle" } });
  try {
    const header = parseTtbHeader(await fetchRange(url, 0, TTB_HEADER_BYTES));
    const compressed = await fetchRange(url, header.indexOffset, header.indexGzLength);
    bundle = TtbIndex.parse(decoder.decode(gunzipSync(compressed)));
    bundleState = "ready";
    missing.clear();
    log("note", `bundle ready: ${bundle.size.toLocaleString()} files available on demand`);
  } catch (error) {
    bundleState = "failed";
    log("error", `could not load the bundle index from ${url} (${errorMessage(error)}), only the pack is available`);
  }
  if (missedAwaitingIndex) {
    missedAwaitingIndex = false;
    // The session caches misses too, so only a reopen looks again.
    if (compiledSinceOpen) restartPending = true;
    post({ kind: "recompile" });
  }
  post({ kind: "progress", progress: { phase: "ready" } });
}

async function flushUnsaved(): Promise<void> {
  if (!assets || !options?.bundleUrl || unsaved.size === 0) return;
  const pending = [...unsaved];
  unsaved.clear();
  for (const [name, bytes] of pending) await assets.remember(options.bundleUrl, name, bytes);
}

function call(request: object): RawResponse {
  return wasm().call(request) as RawResponse;
}

/** Local wall-clock seconds for `\today`, taken once per session so output stays reproducible. */
function wallClock(): number {
  const now = new Date();
  return Math.floor(now.getTime() / 1000) - now.getTimezoneOffset() * 60;
}

function openSession(): void {
  const { primary, raster } = options!;
  const opened = call({ op: "open", format, primary, raster, buildDate: wallClock() });
  if (opened.status === "error") throw new Error(opened.message);
  restartPending = false;
  compiledSinceOpen = false;
}

async function init(opts: WorkerOptions): Promise<void> {
  options = opts;
  if (opts.bundleUrl) bundleState = "pending";
  assets = await AssetCache.open(typeof caches === "undefined" ? undefined : caches);

  post({ kind: "progress", progress: { phase: "fetching-module" } });
  const source = assets ? await assets.revalidating(opts.moduleUrl) : await fetch(opts.moduleUrl);
  if (!source.ok) throw new Error(`could not fetch ${opts.moduleUrl}: HTTP ${source.status}`);

  post({ kind: "progress", progress: { phase: "instantiating" } });
  engine = await Engine.start(
    async (imports) =>
      (
        await WebAssembly.instantiateStreaming(source, imports).catch(
          // Servers that do not send application/wasm.
          async () => WebAssembly.instantiate(await (await fetch(opts.moduleUrl)).arrayBuffer(), imports),
        )
      ).instance,
    {
      readFile,
      readSource: (path) => projectSources.get(path)?.text,
      log,
    },
  );

  format = await loadPack(opts.packUrl);
  if (assets && opts.bundleUrl) {
    for (const [name, bytes] of await assets.restore(opts.bundleUrl)) {
      if (!resident.has(name)) resident.set(name, bytes);
    }
  }

  openSession();
  post({ kind: "progress", progress: { phase: "ready" } });

  if (opts.bundleUrl) void indexBundle(opts.bundleUrl);
}

/** Make every file in the support pack resident, and return the format's name. */
async function loadPack(url: string): Promise<string> {
  const response = assets ? await assets.revalidating(url) : await fetch(url);
  if (!response.ok || !response.body) {
    throw new Error(`could not fetch the support pack at ${url}: HTTP ${response.status}`);
  }

  const total = Number(response.headers.get("content-length")) || undefined;
  const chunks: Uint8Array[] = [];
  let loaded = 0;
  post({ kind: "progress", progress: { phase: "fetching-pack", loaded, total } });
  const reader = response.body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    loaded += value.length;
    post({ kind: "progress", progress: { phase: "fetching-pack", loaded, total } });
  }
  const archive = new Uint8Array(loaded);
  let at = 0;
  for (const chunk of chunks) {
    archive.set(chunk, at);
    at += chunk.length;
  }

  let files: Map<string, Uint8Array>;
  try {
    files = untar(await inflate(archive));
  } catch (error) {
    throw new Error(`${url} is not a TeXyoN support pack (${errorMessage(error)})`);
  }

  const manifestBytes = files.get("manifest.json");
  if (!manifestBytes) throw new Error(`the support pack at ${url} has no manifest`);
  const manifest = JSON.parse(decoder.decode(manifestBytes)) as {
    format: string;
    files: Record<string, number>;
  };
  for (const [name, size] of Object.entries(manifest.files)) {
    const bytes = files.get(name);
    if (bytes?.length !== size) throw new Error(`the support pack at ${url} is missing ${name}`);
    resident.set(name, bytes);
  }
  return manifest.format;
}

async function takeRasters(raw: RawRaster[], scale: number): Promise<PageRaster[]> {
  const { exports } = wasm();
  // createImageBitmap copies the ImageData synchronously, so each page is freed at once.
  const pending = raw.map(({ index, ptr, len }) => {
    const header = new DataView(exports.memory.buffer, ptr, 8);
    const pixels = new Uint8ClampedArray(exports.memory.buffer, ptr + 8, len - 8);
    const bitmap = createImageBitmap(
      new ImageData(pixels, header.getUint32(0, true), header.getUint32(4, true)),
    );
    exports.texyon_wasm_free(ptr, len);
    return { index, bitmap };
  });
  return Promise.all(pending.map(async ({ index, bitmap }) => ({ index, scale, bitmap: await bitmap })));
}

function collect(report: RawCompileReport, elapsedMs: number, rasters: PageRaster[]): CompileResult {
  const pages = report.pages.map(({ index, ptr, len }) => ({
    index,
    svg: decoder.decode(wasm().take({ ptr, len })),
  }));
  return {
    published: report.published,
    pages,
    rasters,
    changed: report.changed,
    removed: report.removed,
    pageSizes: report.pageSizes,
    pageOverlays: report.pageOverlays,
    pageCount: report.pageCount,
    log: report.log,
    outcome: report.outcome,
    aborted: report.aborted,
    provisional: missedAwaitingIndex,
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

function unexpected(reply: RawResponse): Reply {
  return reply.status === "error"
    ? { kind: "error", message: reply.message }
    : { kind: "error", message: `the engine answered ${reply.status}` };
}

async function handle(request: WorkerRequest): Promise<[Reply, Transferable[]?]> {
  switch (request.kind) {
    case "init":
      await init(request.options);
      return [{ kind: "ok" }];

    case "resources":
      for (const [path, bytes] of Object.entries(request.upserts)) projectResources.set(path, bytes);
      for (const path of request.removed) projectResources.delete(path);
      missing.clear();
      if (compiledSinceOpen) restartPending = true;
      return [{ kind: "ok" }];

    case "compile":
    case "continueTail":
    case "fullPass": {
      let reply: RawResponse;
      const started = performance.now();
      if (request.kind === "compile") {
        // Applied before anything can fail, so the page and the Worker agree on the sources.
        for (const [path, edit] of Object.entries(request.edits)) {
          const held = projectSources.get(path);
          if (held?.revision !== edit.base) {
            throw new Error(`an edit to ${path} is against a revision the Worker does not hold`);
          }
          projectSources.set(path, { text: applyEdit(held.text, edit), revision: edit.revision });
        }
        for (const [path, source] of Object.entries(request.upserts)) projectSources.set(path, source);
        for (const path of request.removed) projectSources.delete(path);

        if (restartPending) {
          call({ op: "shutdown" });
          openSession();
        }

        const files: Record<string, number> = {};
        for (const [path, { revision }] of projectSources) files[path] = revision;
        rasterScale = request.scale;
        reply = call({
          op: "compile",
          files,
          edits: request.edits,
          viewport: request.viewport,
          scale: request.scale,
        });
      } else {
        reply = call({ op: request.kind });
      }
      const elapsed = performance.now() - started;

      if (reply.status === "idle") return [{ kind: "idle" }];
      if (reply.status !== "compiled" && reply.status !== "passed") return [unexpected(reply)];
      compiledSinceOpen = true;
      const rasters = await takeRasters(reply.rasters, rasterScale ?? 0);
      const result = collect(reply, elapsed, rasters);
      void flushUnsaved();
      const bitmaps = rasters.map(({ bitmap }) => bitmap);
      return reply.status === "passed"
        ? [{ kind: "passed", result, changed: reply.rerun }, bitmaps]
        : [{ kind: "compiled", result }, bitmaps];
    }

    case "raster": {
      const reply = call({ op: "raster", pages: request.pages, scale: request.scale });
      if (reply.status !== "rastered") return [unexpected(reply)];
      const rasters = await takeRasters(reply.pages, request.scale);
      return [{ kind: "rastered", rasters }, rasters.map(({ bitmap }) => bitmap)];
    }

    case "export": {
      const started = performance.now();
      const reply = call({ op: "export", now: Math.floor(Date.now() / 1000) });
      if (reply.status !== "exported") return [unexpected(reply)];
      const pdf = wasm().take(reply);
      return [{ kind: "exported", pdf, elapsedMs: performance.now() - started }, [pdf.buffer]];
    }

    case "commands": {
      const reply = call({ op: "commands" });
      if (reply.status !== "commands") return [unexpected(reply)];
      return [{ kind: "commands", commands: reply.commands }];
    }

    case "forwardSearch":
    case "backwardSearch": {
      const reply =
        request.kind === "forwardSearch"
          ? call({ op: request.kind, path: request.path, line: request.line })
          : call({ op: request.kind, pageIndex: request.pageIndex, hBp: request.hBp, vBp: request.vBp });
      if (reply.status !== "located") return [unexpected(reply)];
      const located = reply.rect && { rect: reply.rect, boxes: reply.boxes ?? [] };
      return [{ kind: "located", located, source: reply.source }];
    }

    case "cancel":
    case "shutdown":
      call({ op: request.kind });
      return [{ kind: "ok" }];
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const { id } = event.data;
  try {
    const [reply, transfer] = await handle(event.data);
    post({ ...reply, id }, transfer);
  } catch (error) {
    const message =
      error instanceof WasiExit ? `${error.message}, restart the engine` : errorMessage(error);
    post({ kind: "error", message, id });
  }
};
