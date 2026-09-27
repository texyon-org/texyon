/**
 * Tectonic in the browser: a resident LaTeX engine you can type into.
 *
 * ```ts
 * const engine = await TectonicEngine.create({
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
  LoadProgress,
  LogLevel,
  LogLine,
  Link,
  Located,
  Outcome,
  PageOverlay,
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
  Located,
  LoadProgress,
  LogLine,
  ProjectFiles,
  SourceSpot,
  WorkerRequest,
  WorkerResponse,
} from "./protocol.js";

/** Thrown when the engine could not do what was asked. */
export class TectonicError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TectonicError";
  }
}

/** A resident LaTeX engine running in a Worker. */
export class TectonicEngine {
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

  /** Binaries as last sent to the Worker, diffed by identity. */
  #sentResources = new Map<string, Uint8Array>();

  /**
   * Text sources as last sent to the Worker, diffed by identity.
   *
   * Sound, not just probable: strings are immutable, so changed content is
   * always a changed reference. A rebuilt-but-identical string only costs a
   * harmless resend, which the Worker recognizes and ignores.
   */
  #sentSources = new Map<string, string>();

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
        pending.reject(new TectonicError(message.message));
      } else if (message.kind === "compiled") {
        (pending.resolve as (value: CompileResult) => void)(message.result);
      } else if (message.kind === "exported") {
        (pending.resolve as (value: { pdf: Uint8Array<ArrayBuffer>; elapsedMs: number }) => void)({
          pdf: message.pdf,
          elapsedMs: message.elapsedMs,
        });
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
      const error = new TectonicError(event.message || "the engine Worker failed");
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
  static async create(options: EngineOptions = {}): Promise<TectonicEngine> {
    const worker = new Worker(options.workerUrl ?? new URL("./worker.js", import.meta.url), {
      type: "module",
      name: "tectonic",
    });

    const engine = new TectonicEngine(worker, options);

    // Resolve the asset URLs here rather than in the Worker. A relative URL
    // inside a Worker resolves against the *worker script*, which a bundler is
    // free to put anywhere — so "tectonic/" would silently mean something
    // different from what the caller wrote.
    const base = typeof document !== "undefined" ? document.baseURI : location.href;
    const absolute = (url: string) => new URL(url, base).href;

    await engine.#send({
      id: 0, // replaced by #send
      kind: "init",
      options: {
        moduleUrl: absolute(options.moduleUrl ?? "tectonic/tectonic_wasm.wasm"),
        supportUrl: absolute(options.supportUrl ?? "tectonic/"),
        bundleUrl: options.bundleUrl ? absolute(options.bundleUrl) : undefined,
        primary: options.primary ?? "main.tex",
        prefetch: options.prefetch ?? true,
      },
    });

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
   * Calls are serialized: a compile issued while another is running waits for
   * it. To abandon the running one instead, call {@link cancel} first.
   */
  async compile(
    files: string | ProjectFiles,
    viewport?: [number, number],
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
    const sourceUpserts: Record<string, string> = {};
    for (const [path, text] of Object.entries(sources)) {
      if (this.#sentSources.get(path) !== text) {
        sourceUpserts[path] = text;
        this.#sentSources.set(path, text);
      }
    }
    const sourceRemoved = [...this.#sentSources.keys()].filter((path) => !(path in sources));
    for (const path of sourceRemoved) this.#sentSources.delete(path);

    return this.#queue(async () => {
      if (Object.keys(upserts).length > 0 || removed.length > 0) {
        await this.#send<void>({ id: 0, kind: "resources", upserts, removed });
        for (const [path, bytes] of Object.entries(upserts)) {
          this.#sentResources.set(path, bytes);
        }
        for (const path of removed) this.#sentResources.delete(path);
      }

      return this.#send<CompileResult>({
        id: 0,
        kind: "compile",
        upserts: sourceUpserts,
        removed: sourceRemoved,
        viewport,
      });
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
   * Finish a viewport-bounded compile through the end of the document.
   *
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
   * This is the preview's own output run through xdvipdfmx — the same XDV the
   * viewer is showing, not a second compile — so a download cannot disagree
   * with what was on screen.
   *
   * A viewport-bounded compile stops at the last visible page, and exporting
   * that would silently drop the tail. So this settles the document first and
   * hands back the frames that took, in {@link ExportResult.tail}; apply them
   * as you would a compile result to keep the viewer in step. Usually it is
   * empty, because the document was already finished.
   */
  async exportPdf(): Promise<ExportResult> {
    return this.#queue(async () => {
      const tail: CompileResult[] = [];

      // Settle through the end of the document. Each continuation reports
      // whether more is pending, so this terminates on the engine's own
      // account rather than a fixed bound.
      for (;;) {
        const frame = await this.#send<CompileResult | null>({
          id: 0,
          kind: "continueTail",
        });
        if (!frame) break;
        tail.push(frame);
        if (!frame.tailPending) break;
      }

      const { pdf, elapsedMs } = await this.#send<{
        pdf: Uint8Array<ArrayBuffer>;
        elapsedMs: number;
      }>({ id: 0, kind: "export" });

      return { pdf, tail, elapsedMs };
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
    const result = this.#inFlight.then(operation, operation);
    // Keep the chain alive even when a compile rejects.
    this.#inFlight = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  #send<T>(request: WorkerRequest): Promise<T> {
    if (this.#closed) {
      return Promise.reject(new TectonicError("the engine has been closed"));
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
