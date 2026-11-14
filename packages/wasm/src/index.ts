/**
 * TeXyon in the browser: a resident LaTeX engine you can type into.
 *
 * ```ts
 * const engine = await TexyonEngine.create({
 *   onProgress: (p) => console.log(p.phase),
 * });
 *
 * // A patch: SVG for the pages whose appearance actually moved.
 * const result = await engine.compile(source);
 * for (const page of result.pages) slots[page.index].innerHTML = page.svg;
 *
 * // And the same typesetting as a PDF, when someone wants to keep it.
 * const { pdf } = await engine.exportPdf();
 * ```
 *
 * The engine is *resident*: it keeps one document open and applies each new
 * version of the text to it. That is what makes it usable behind a keystroke.
 * A cold LaTeX run spends almost all of its time before your cursor — reading a
 * 24 MB format dump, loading the class, running the preamble — and a resident
 * session pays that once, then rewinds to a checkpoint just before your edit
 * instead of starting over. {@link CompileStats.replayRung} reports how deep
 * that was; anything above zero means the preamble was not re-run.
 *
 * Everything runs in a Worker, so the page stays responsive even while TeX is
 * blocked on a support file.
 */

export type {
  CompileResult,
  CompileStats,
  Destination,
  Diagnostic,
  EngineCommand,
  EngineOptions,
  ExportResult,
  ExportStep,
  LoadProgress,
  LogLevel,
  LogLine,
  Link,
  Located,
  Outcome,
  PageOverlay,
  PageRaster,
  ProjectFiles,
  Severity,
  SourceSpot,
  SyncRect,
} from "./protocol.js";

import type {
  CompileResult,
  EngineCommand,
  EngineOptions,
  ExportResult,
  ExportStep,
  Located,
  LoadProgress,
  LogLine,
  PageRaster,
  ProjectFiles,
  SourceSplice,
  SourceSpot,
  SourceUpsert,
  WorkerRequest,
  WorkerResponse,
} from "./protocol.js";
import { spliceBetween } from "./splice.js";

/** Full passes an export may take. LaTeX rarely needs more than three, and some documents never settle. */
const MAX_PASSES = 5;

/** Thrown when the engine could not do what was asked. */
export class TexyonError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TexyonError";
  }
}

/**
 * A URL the page may start a Worker from.
 *
 * Browsers refuse a cross-origin Worker script, which is what a package served
 * from a CDN or a separate dev server is. A same-origin module that imports it
 * only needs CORS.
 */
function sameOriginScript(url: URL): string {
  if (url.origin === location.origin) return url.href;
  const shim = `import ${JSON.stringify(url.href)};`;
  return URL.createObjectURL(new Blob([shim], { type: "text/javascript" }));
}

/** A resident LaTeX engine running in a Worker. */
export class TexyonEngine {
  #worker: Worker;
  #pending = new Map<
    number,
    { resolve: (value: never) => void; reject: (error: Error) => void }
  >();
  #nextId = 1;
  #onProgress?: (progress: LoadProgress) => void;
  #onLog?: (line: LogLine) => void;
  #onRecompileNeeded?: () => void;
  #primary: string;
  #closed = false;

  /** In-flight compile, so a new one can supersede it. */
  #inFlight: Promise<unknown> = Promise.resolve();
  /** Queued operations that have not settled yet. */
  #queued = 0;

  /** Binaries as last sent to the Worker, diffed by identity. */
  #sentResources = new Map<string, Uint8Array>();

  /**
   * Text sources as last sent to the Worker, with the revision they were
   * sent at, diffed by identity.
   *
   * Sound, not just probable: strings are immutable, so changed content is
   * always a changed reference. A rebuilt-but-identical string only costs a
   * harmless resend, which the Worker recognizes and ignores.
   */
  #sentSources = new Map<string, SourceUpsert>();
  #nextSourceRevision = 1;

  private constructor(worker: Worker, options: EngineOptions) {
    this.#worker = worker;
    this.#onProgress = options.onProgress;
    this.#onLog = options.onLog;
    this.#onRecompileNeeded = options.onRecompileNeeded;
    this.#primary = options.primary ?? "main.tex";

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;

      if (message.kind === "progress") {
        this.#onProgress?.(message.progress);
        return;
      }

      if (message.kind === "log") {
        this.#onLog?.(message.line);
        return;
      }

      if (message.kind === "recompile") {
        this.#onRecompileNeeded?.();
        return;
      }

      const pending = this.#pending.get(message.id);
      if (!pending) return;
      this.#pending.delete(message.id);

      if (message.kind === "error") {
        pending.reject(new TexyonError(message.message));
      } else if (message.kind === "compiled") {
        (pending.resolve as (value: CompileResult) => void)(message.result);
      } else if (message.kind === "passed") {
        (pending.resolve as (value: { result: CompileResult; changed: boolean }) => void)({
          result: message.result,
          changed: message.changed,
        });
      } else if (message.kind === "exported") {
        (pending.resolve as (value: { pdf: Uint8Array<ArrayBuffer>; elapsedMs: number }) => void)({
          pdf: message.pdf,
          elapsedMs: message.elapsedMs,
        });
      } else if (message.kind === "rastered") {
        (pending.resolve as (value: PageRaster[]) => void)(message.rasters);
      } else if (message.kind === "commands") {
        (pending.resolve as (value: { commands: EngineCommand[] }) => void)({
          commands: message.commands,
        });
      } else if (message.kind === "located") {
        (
          pending.resolve as (value: {
            located: Located | null;
            source: SourceSpot | null;
          }) => void
        )({ located: message.located, source: message.source });
      } else if (message.kind === "idle") {
        (pending.resolve as (value: null) => void)(null);
      } else {
        (pending.resolve as (value: void) => void)();
      }
    };

    worker.onerror = (event) => {
      const error = new TexyonError(event.message || "the engine Worker failed");
      for (const pending of this.#pending.values()) pending.reject(error);
      this.#pending.clear();
    };
  }

  /**
   * Start an engine and wait until it can typeset.
   *
   * Resolves once the wasm module is instantiated and the format dump is
   * loaded. Support files keep arriving in the background afterwards; watch
   * {@link EngineOptions.onProgress} if you want to show that.
   */
  static async create(options: EngineOptions = {}): Promise<TexyonEngine> {
    // Resolve the asset URLs here rather than in the Worker. A relative URL
    // inside a Worker resolves against the *worker script*, which a bundler is
    // free to put anywhere — so "tectonic/" would silently mean something
    // different from what the caller wrote.
    const base = typeof document !== "undefined" ? document.baseURI : location.href;
    const absolute = (url: string) => new URL(url, base).href;

    const workerUrl =
      options.workerUrl !== undefined
        ? new URL(options.workerUrl, base)
        : new URL("./worker.js", import.meta.url);
    const script = sameOriginScript(workerUrl);
    const worker = new Worker(script, { type: "module", name: "texyon" });

    const engine = new TexyonEngine(worker, options);

    await engine.#send({
      id: 0, // replaced by #send
      kind: "init",
      options: {
        // Literal URLs, so bundlers copy the shipped files into the build.
        moduleUrl:
          options.moduleUrl !== undefined
            ? absolute(options.moduleUrl)
            : new URL("./texyon_wasm.wasm", import.meta.url).href,
        packUrl:
          options.packUrl !== undefined
            ? absolute(options.packUrl)
            : new URL("./texyon-pack.tar.gz", import.meta.url).href,
        bundleUrl: options.bundleUrl ? absolute(options.bundleUrl) : undefined,
        primary: options.primary ?? "main.tex",
        raster: options.raster ?? false,
      },
    });
    if (script !== workerUrl.href) URL.revokeObjectURL(script);

    return engine;
  }

  /**
   * Typeset a project.
   *
   * `files` is the project's complete current state, keyed by path relative to
   * its root, and must contain the primary named in {@link
   * EngineOptions.primary}. A bare string is shorthand for a project that is
   * only its primary.
   *
   * Passing everything every time is the intended usage and the cheap one: the
   * engine diffs each file against what it last saw, so untouched files cost
   * nothing and the one you edited replays only from the point that changed.
   *
   * `viewport` names the visible page range, zero-based and inclusive. Passing
   * it bounds the foreground work to those pages and leaves the rest for
   * {@link continueTail}, which is what keeps latency flat on a long document.
   *
   * `scale`, in pixels per big point, has a raster engine draw the changed pages
   * around `viewport` into {@link CompileResult.rasters}. Its later
   * {@link continueTail} frames draw at the same scale.
   *
   * Calls are serialized: a compile issued while another is running waits for
   * it. To abandon the running one instead, call {@link cancel} first.
   */
  async compile(
    files: string | ProjectFiles,
    viewport?: [number, number],
    scale?: number,
  ): Promise<CompileResult> {
    const project =
      typeof files === "string" ? { [this.#primary]: files } : files;

    const sources: Record<string, string> = {};
    const binaries = new Map<string, Uint8Array>();
    for (const [path, content] of Object.entries(project)) {
      if (typeof content === "string") sources[path] = content;
      else binaries.set(path, content);
    }

    const upserts: Record<string, Uint8Array> = {};
    for (const [path, bytes] of binaries) {
      if (this.#sentResources.get(path) !== bytes) upserts[path] = bytes;
    }
    const removed = [...this.#sentResources.keys()].filter((path) => !binaries.has(path));

    // Only what moved crosses the boundary; the Worker holds the rest. Updated
    // here rather than after the send so a second compile computed before the
    // first is posted diffs against what will already be on the wire.
    // A known file goes as a splice, so a keystroke in a long document does
    // not copy all of it into the Worker and the engine.
    const sourceUpserts: Record<string, SourceUpsert> = {};
    const sourceEdits: Record<string, SourceSplice> = {};
    for (const [path, text] of Object.entries(sources)) {
      const sent = this.#sentSources.get(path);
      if (sent?.text === text) continue;
      const revision = this.#nextSourceRevision++;
      if (sent === undefined) sourceUpserts[path] = { text, revision };
      else sourceEdits[path] = { ...spliceBetween(sent.text, text), base: sent.revision, revision };
      this.#sentSources.set(path, { text, revision });
    }
    const sourceRemoved = [...this.#sentSources.keys()].filter((path) => !(path in sources));
    for (const path of sourceRemoved) this.#sentSources.delete(path);

    return this.#queue(async () => {
      try {
        if (Object.keys(upserts).length > 0 || removed.length > 0) {
          await this.#send<void>({ id: 0, kind: "resources", upserts, removed });
          for (const [path, bytes] of Object.entries(upserts)) {
            this.#sentResources.set(path, bytes);
          }
          for (const path of removed) this.#sentResources.delete(path);
        }

        return await this.#send<CompileResult>({
          id: 0,
          kind: "compile",
          upserts: sourceUpserts,
          edits: sourceEdits,
          removed: sourceRemoved,
          viewport,
          scale,
        });
      } catch (error) {
        // The Worker may not hold what was recorded as sent, and edits
        // against it would be refused, so the next compile sends every source whole.
        this.#sentSources.clear();
        throw error;
      }
    });
  }

  /**
   * Every command the document currently has defined.
   *
   * The engine's own account, so it covers what the preamble defined, what each
   * package defined under the options it was given, and anything built at run
   * time that no scan of a `.sty` file could have seen. Empty before the first
   * compile, when there is no document open to ask about.
   *
   * What it cannot say is what a command's arguments are *called* — TeX stores
   * `#1`, not a name — so an editor wanting that has to pair this with a
   * description from somewhere else.
   */
  async commands(): Promise<EngineCommand[]> {
    const { commands } = await this.#send<{ commands: EngineCommand[] }>({
      id: 0,
      kind: "commands",
    });
    return commands;
  }

  /**
   * Find where a source line landed on the page, and the boxes to flash there.
   *
   * `path` is spelled the way you spell it in {@link compile}. A line that
   * produced nothing, such as a comment or a line inside a beamer frame, falls
   * back to the nearest line that did, looking below it first. Resolves to
   * `null` when nothing near it reached the page.
   */
  async forwardSearch(path: string, line: number): Promise<Located | null> {
    const { located } = await this.#send<{ located: Located | null }>({
      id: 0,
      kind: "forwardSearch",
      path,
      line,
    });
    return located;
  }

  /**
   * Find which source line produced a point on a page.
   *
   * The point is in PostScript big points from the page's top-left corner —
   * the same units {@link CompileResult.pageSizes} reports. The path comes back
   * spelled the way you spelled it.
   */
  async backwardSearch(
    pageIndex: number,
    hBp: number,
    vBp: number,
  ): Promise<SourceSpot | null> {
    const { source } = await this.#send<{ source: SourceSpot | null }>({
      id: 0,
      kind: "backwardSearch",
      pageIndex,
      hBp,
      vBp,
    });
    return source;
  }

  /**
   * Draw published pages of a raster engine at `scale` pixels per big point.
   *
   * For pages that scrolled into view, or a new zoom. Pages a compile changed
   * near its viewport arrive with the compile instead.
   */
  async raster(pages: number[], scale: number): Promise<PageRaster[]> {
    if (pages.length === 0) return [];
    return this.#queue(() =>
      this.#send<PageRaster[]>({ id: 0, kind: "raster", pages, scale }),
    );
  }

  /**
   * Continue a viewport-bounded compile toward the end of the document.
   *
   * Each call does a short chunk of the work and reports `tailPending` while
   * more remains, so a caller can stop between calls when the text changes.
   * Returns `null` when there was nothing pending. Worth calling when the user
   * pauses: it settles the pages below the fold so scrolling does not have to
   * wait for them.
   */
  async continueTail(): Promise<CompileResult | null> {
    return this.#queue(() => this.#send<CompileResult | null>({ id: 0, kind: "continueTail" }));
  }

  /**
   * Produce a PDF of the current document.
   *
   * The preview replays from the nearest checkpoint, so it never reads its
   * .aux again and its references can lag. This typesets the whole document
   * from the top until the files it reads back stop changing, like running
   * LaTeX until it stops asking for a rerun, then runs the result through
   * xdvipdfmx. The passes' frames come back in {@link ExportResult.tail};
   * apply them as you would a compile result to keep the viewer in step.
   *
   * Each pass is a whole compile, so `onStep` says which one is running.
   */
  async exportPdf(onStep?: (step: ExportStep) => void): Promise<ExportResult> {
    return this.#queue(async () => {
      const tail: CompileResult[] = [];
      let unsettled = true;
      for (let pass = 0; pass < MAX_PASSES && unsettled; pass++) {
        onStep?.({ phase: "pass", pass: pass + 1 });
        const { result, changed } = await this.#send<{ result: CompileResult; changed: boolean }>({
          id: 0,
          kind: "fullPass",
        });
        tail.push(result);
        if (result.aborted) throw new TexyonError(`the document did not finish: ${result.aborted}`);
        unsettled = changed;
      }

      onStep?.({ phase: "writing" });
      const { pdf, elapsedMs } = await this.#send<{
        pdf: Uint8Array<ArrayBuffer>;
        elapsedMs: number;
      }>({ id: 0, kind: "export" });

      return { pdf, tail, unsettled, elapsedMs };
    });
  }

  /**
   * Ask the running compile to stop at its next cancellation point.
   *
   * The engine rewinds to its last good state, so the previous preview stays
   * valid. Use this when the user types again before the last compile finished.
   */
  async cancel(): Promise<void> {
    await this.#send<void>({ id: 0, kind: "cancel" });
  }

  /** Shut the engine down and terminate its Worker. */
  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;

    try {
      await this.#send<void>({ id: 0, kind: "shutdown" });
    } catch {
      // A shutdown that fails still ends with a terminated Worker.
    }

    this.#worker.terminate();
  }

  /** Run `operation` after whatever is already queued. */
  #queue<T>(operation: () => Promise<T>): Promise<T> {
    // With nothing ahead, start now: a later microtask would post the request
    // only after the render this same keystroke scheduled.
    const result = this.#queued === 0 ? operation() : this.#inFlight.then(operation, operation);
    this.#queued++;
    // Keep the chain alive even when a compile rejects.
    this.#inFlight = result.then(
      () => void this.#queued--,
      () => void this.#queued--,
    );
    return result;
  }

  #send<T>(request: WorkerRequest): Promise<T> {
    if (this.#closed) {
      return Promise.reject(new TexyonError("the engine has been closed"));
    }

    const id = this.#nextId++;

    return new Promise<T>((resolve, reject) => {
      this.#pending.set(id, {
        resolve: resolve as (value: never) => void,
        reject,
      });
      this.#worker.postMessage({ ...request, id });
    });
  }
}
