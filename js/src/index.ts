/**
 * Tectonic in the browser: a resident LaTeX engine you can type into.
 *
 * ```ts
 * const engine = await TectonicEngine.create({
 *   onProgress: (p) => console.log(p.phase),
 * });
 *
 * const first = await engine.compile(source);
 * document.querySelector("embed")!.src = URL.createObjectURL(
 *   new Blob([first.pdf!], { type: "application/pdf" }),
 * );
 * ```
 *
 * The engine is *resident*: it keeps one document open and applies each new
 * version of the text to it. That is what makes it usable behind a keystroke.
 * A cold LaTeX run spends almost all of its time before your cursor — reading a
 * 24 MB format dump, loading the class, running the preamble — and a resident
 * session pays that once, then rewinds to a checkpoint just before your edit
 * instead of starting over. {@link CompileStats.spliceAt} reports how many
 * bytes of the previous run it managed to keep.
 *
 * Everything runs in a Worker, so the page stays responsive even while TeX is
 * blocked on a support file.
 */

export type {
  CompileResult,
  CompileStats,
  EngineOptions,
  LoadProgress,
  LogLevel,
  LogLine,
  Outcome,
} from "./protocol";

import type {
  CompileResult,
  EngineOptions,
  LoadProgress,
  LogLine,
  WorkerRequest,
  WorkerResponse,
} from "./protocol";

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
  #closed = false;

  /** In-flight compile, so a new one can supersede it. */
  #inFlight: Promise<unknown> = Promise.resolve();

  private constructor(worker: Worker, options: EngineOptions) {
    this.#worker = worker;
    this.#onProgress = options.onProgress;
    this.#onLog = options.onLog;

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

      const pending = this.#pending.get(message.id);
      if (!pending) return;
      this.#pending.delete(message.id);

      if (message.kind === "error") {
        pending.reject(new TectonicError(message.message));
      } else if (message.kind === "compiled") {
        (pending.resolve as (value: CompileResult) => void)(message.result);
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
   * Typeset this text.
   *
   * The text is the document's complete current contents — the engine diffs it
   * against what it last saw and replays only from the point that changed, so
   * sending the whole buffer is both the intended usage and the cheap one.
   *
   * `viewport` names the visible page range, zero-based and inclusive. Passing
   * it bounds the foreground work to those pages and leaves the rest for
   * {@link continueTail}, which is what keeps latency flat on a long document.
   *
   * Calls are serialized: a compile issued while another is running waits for
   * it. To abandon the running one instead, call {@link cancel} first.
   */
  async compile(text: string, viewport?: [number, number]): Promise<CompileResult> {
    return this.#queue(() =>
      this.#send<CompileResult>({ id: 0, kind: "compile", text, viewport }),
    );
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
