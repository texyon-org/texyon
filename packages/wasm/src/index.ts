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
  RenderedPage,
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
  PageRaster,
  ProjectFiles,
  Reply,
  Request,
  SourceSplice,
  SourceSpot,
  SourceUpsert,
  WorkerResponse,
} from "./protocol.js";
import { spliceBetween } from "./splice.js";

/** LaTeX rarely needs more than three passes, and some documents never settle. */
const MAX_PASSES = 5;

type ReplyOf<K extends Reply["kind"]> = Extract<Reply, { kind: K }>;

/** Thrown when the engine could not do what was asked. */
export class TexyonError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TexyonError";
  }
}

/** Browsers refuse cross-origin Worker scripts, but a same-origin module may import one. */
function sameOriginScript(url: URL): string {
  if (url.origin === location.origin) return url.href;
  const shim = `import ${JSON.stringify(url.href)};`;
  return URL.createObjectURL(new Blob([shim], { type: "text/javascript" }));
}

/** A resident LaTeX engine running in a Worker. */
export class TexyonEngine {
  #worker: Worker;
  #pending = new Map<number, { resolve: (reply: Reply) => void; reject: (error: Error) => void }>();
  #nextId = 1;
  #primary: string;
  #closed = false;
  #inFlight: Promise<unknown> = Promise.resolve();
  #queued = 0;
  /** Binaries as last sent, compared by identity. */
  #sentResources = new Map<string, Uint8Array>();
  /** Sources as last sent, compared by identity. */
  #sentSources = new Map<string, SourceUpsert>();
  #nextSourceRevision = 1;

  private constructor(worker: Worker, options: EngineOptions) {
    this.#worker = worker;
    this.#primary = options.primary ?? "main.tex";

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      switch (message.kind) {
        case "progress":
          options.onProgress?.(message.progress);
          return;
        case "log":
          options.onLog?.(message.line);
          return;
        case "recompile":
          options.onRecompileNeeded?.();
          return;
      }
      const pending = this.#pending.get(message.id);
      if (!pending) return;
      this.#pending.delete(message.id);
      pending.resolve(message);
    };

    worker.onerror = (event) => {
      const error = new TexyonError(event.message || "the engine Worker failed");
      for (const pending of this.#pending.values()) pending.reject(error);
      this.#pending.clear();
    };
  }

  /** Start an engine and wait until it can typeset. */
  static async create(options: EngineOptions = {}): Promise<TexyonEngine> {
    // Resolved here, since inside the Worker a relative URL means relative to its script.
    const base = typeof document !== "undefined" ? document.baseURI : location.href;
    const absolute = (url: string) => new URL(url, base).href;

    const workerUrl =
      options.workerUrl !== undefined
        ? new URL(options.workerUrl, base)
        : new URL("./worker.js", import.meta.url);
    const script = sameOriginScript(workerUrl);
    const engine = new TexyonEngine(new Worker(script, { type: "module", name: "texyon" }), options);

    try {
      await engine.#call(
        {
          kind: "init",
          options: {
            // Literal `new URL` calls, so bundlers copy the packaged files.
            moduleUrl:
              options.moduleUrl !== undefined
                ? absolute(options.moduleUrl)
                : new URL("./texyon_wasm.wasm", import.meta.url).href,
            packUrl:
              options.packUrl !== undefined
                ? absolute(options.packUrl)
                : new URL("./texyon-pack.tar.gz", import.meta.url).href,
            bundleUrl: options.bundleUrl ? absolute(options.bundleUrl) : undefined,
            primary: engine.#primary,
            raster: options.raster ?? false,
          },
        },
        "ok",
      );
    } finally {
      if (script !== workerUrl.href) URL.revokeObjectURL(script);
    }
    return engine;
  }

  /**
   * Typeset the project's complete current files, or just the primary's text.
   *
   * `viewport` bounds the work to an inclusive page range, leaving the rest to
   * {@link continueTail}. `scale`, in pixels per big point, has a raster engine
   * draw the changed pages near it. Calls run in order.
   */
  async compile(
    files: string | ProjectFiles,
    viewport?: [number, number],
    scale?: number,
  ): Promise<CompileResult> {
    const project = typeof files === "string" ? { [this.#primary]: files } : files;

    const sources = new Map<string, string>();
    const binaries = new Map<string, Uint8Array>();
    for (const [path, content] of Object.entries(project)) {
      if (typeof content === "string") sources.set(path, content);
      else binaries.set(path, content);
    }

    const upserts: Record<string, Uint8Array> = {};
    for (const [path, bytes] of binaries) {
      if (this.#sentResources.get(path) !== bytes) upserts[path] = bytes;
    }
    const removed = [...this.#sentResources.keys()].filter((path) => !binaries.has(path));

    // Recorded before sending, so a compile queued behind this one diffs against it.
    const sourceUpserts: Record<string, SourceUpsert> = {};
    const sourceEdits: Record<string, SourceSplice> = {};
    for (const [path, text] of sources) {
      const sent = this.#sentSources.get(path);
      if (sent?.text === text) continue;
      const revision = this.#nextSourceRevision++;
      if (sent === undefined) sourceUpserts[path] = { text, revision };
      else sourceEdits[path] = { ...spliceBetween(sent.text, text), base: sent.revision, revision };
      this.#sentSources.set(path, { text, revision });
    }
    const sourceRemoved = [...this.#sentSources.keys()].filter((path) => !sources.has(path));
    for (const path of sourceRemoved) this.#sentSources.delete(path);

    return this.#queue(async () => {
      try {
        if (Object.keys(upserts).length > 0 || removed.length > 0) {
          await this.#call({ kind: "resources", upserts, removed }, "ok");
          for (const [path, bytes] of Object.entries(upserts)) this.#sentResources.set(path, bytes);
          for (const path of removed) this.#sentResources.delete(path);
        }
        const reply = await this.#call(
          {
            kind: "compile",
            upserts: sourceUpserts,
            edits: sourceEdits,
            removed: sourceRemoved,
            viewport,
            scale,
          },
          "compiled",
        );
        return reply.result;
      } catch (error) {
        // The Worker's copies are unknown now, so the next compile sends every source whole.
        this.#sentSources.clear();
        throw error;
      }
    });
  }

  /** Every command the document currently defines. Empty before the first compile. */
  async commands(): Promise<EngineCommand[]> {
    return (await this.#call({ kind: "commands" }, "commands")).commands;
  }

  /** Where a source line landed on the page, or null when nothing near it did. */
  async forwardSearch(path: string, line: number): Promise<Located | null> {
    return (await this.#call({ kind: "forwardSearch", path, line }, "located")).located;
  }

  /** The source line that produced a point, in big points from the page's top-left corner. */
  async backwardSearch(pageIndex: number, hBp: number, vBp: number): Promise<SourceSpot | null> {
    const reply = await this.#call({ kind: "backwardSearch", pageIndex, hBp, vBp }, "located");
    return reply.source;
  }

  /** Draw published pages of a raster engine at `scale` pixels per big point. */
  async raster(pages: number[], scale: number): Promise<PageRaster[]> {
    if (pages.length === 0) return [];
    return this.#queue(
      async () => (await this.#call({ kind: "raster", pages, scale }, "rastered")).rasters,
    );
  }

  /** Typeset the next chunk of a viewport-bounded compile, or return null when none is pending. */
  async continueTail(): Promise<CompileResult | null> {
    return this.#queue(async () => {
      const reply = await this.#call({ kind: "continueTail" }, "compiled", "idle");
      return reply.kind === "compiled" ? reply.result : null;
    });
  }

  /** Typeset the whole document until its auxiliary files settle, then write a PDF. */
  async exportPdf(onStep?: (step: ExportStep) => void): Promise<ExportResult> {
    return this.#queue(async () => {
      const tail: CompileResult[] = [];
      let unsettled = true;
      for (let pass = 1; pass <= MAX_PASSES && unsettled; pass++) {
        onStep?.({ phase: "pass", pass });
        const { result, changed } = await this.#call({ kind: "fullPass" }, "passed");
        tail.push(result);
        if (result.aborted) throw new TexyonError(`the document did not finish: ${result.aborted}`);
        unsettled = changed;
      }
      onStep?.({ phase: "writing" });
      const { pdf, elapsedMs } = await this.#call({ kind: "export" }, "exported");
      return { pdf, tail, unsettled, elapsedMs };
    });
  }

  /** Stop the running compile. The previous preview stays valid. */
  async cancel(): Promise<void> {
    await this.#call({ kind: "cancel" }, "ok");
  }

  /** Shut the engine down and terminate its Worker. */
  async close(): Promise<void> {
    if (this.#closed) return;
    try {
      await this.#call({ kind: "shutdown" }, "ok");
    } catch {
      // Terminated below either way.
    }
    this.#closed = true;
    this.#worker.terminate();
  }

  #queue<T>(operation: () => Promise<T>): Promise<T> {
    // Starting at once when idle posts the request before this keystroke's render.
    const result = this.#queued === 0 ? operation() : this.#inFlight.then(operation, operation);
    this.#queued++;
    this.#inFlight = result.then(
      () => void this.#queued--,
      () => void this.#queued--,
    );
    return result;
  }

  async #call<K extends Reply["kind"]>(request: Request, ...expected: K[]): Promise<ReplyOf<K>> {
    if (this.#closed) throw new TexyonError("the engine has been closed");
    const id = this.#nextId++;
    const reply = await new Promise<Reply>((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#worker.postMessage({ ...request, id });
    });
    if (reply.kind === "error") throw new TexyonError(reply.message);
    if (!isOneOf(reply, expected)) {
      throw new TexyonError(`unexpected ${reply.kind} reply to ${request.kind}`);
    }
    return reply;
  }
}

function isOneOf<K extends Reply["kind"]>(reply: Reply, kinds: K[]): reply is ReplyOf<K> {
  return (kinds as string[]).includes(reply.kind);
}
