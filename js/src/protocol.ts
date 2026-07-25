/**
 * Types shared between the page and the Worker, and with the Rust module.
 *
 * The Rust side of these lives in `crates/wasm/src/protocol.rs`; the two must
 * agree on field names, which are camelCase on both sides.
 */

/** How a compile ended, as TeX itself grades it. */
export type Outcome = "spotless" | "warnings" | "errors";

/** Severity of a line the engine logged. */
export type LogLevel = "note" | "warning" | "error";

/** One diagnostic from the engine. */
export interface LogLine {
  /** How bad it is. */
  level: LogLevel;
  /** What it said. */
  message: string;
}

/** One page of the rendered document. */
export interface RenderedPage {
  /** Zero-based page index. */
  index: number;
  /** Self-contained SVG markup. */
  svg: string;
}

/**
 * What one compile produced.
 *
 * This is a **patch**, not a snapshot. `pages` carries only the pages whose
 * appearance actually changed; everything else the viewer already has is still
 * correct. That is the point of the renderer's fingerprinting — an edit in the
 * last paragraph of a twenty-page document redraws one page.
 */
export interface CompileResult {
  /** SVG for the changed pages. Its indices are exactly {@link changed}. */
  pages: RenderedPage[];
  /** Indices of the pages that changed. */
  changed: number[];
  /** Indices the document no longer has, which the viewer should drop. */
  removed: number[];
  /**
   * Every page's size in PostScript big points.
   *
   * Covers the whole document, not just the changed pages, so a viewer can lay
   * out a page slot before it has the page.
   */
  pageSizes: [number, number][];
  /** Pages in the document. */
  pageCount: number;
  /** TeX's `.log` transcript, in full. */
  log: string;
  /** How TeX graded the run. */
  outcome: Outcome;
  /** A recoverable fatal-class abort, if the engine hit one. */
  aborted?: string;
  /** Whether a viewport-bounded compile left a tail to finish. */
  tailPending: boolean;
  /** Diagnostics gathered during the run. */
  messages: string[];
  /** Timings and incremental-engine statistics. */
  stats: CompileStats;
}

/** What the compile cost, and how much of it was skipped. */
export interface CompileStats {
  /** Milliseconds from request to result, measured in the Worker. */
  elapsedMs: number;
  /** Milliseconds spent in XeTeX. */
  texMs: number;
  /** Milliseconds spent analyzing the XDV. */
  analyzeMs: number;
  /** Milliseconds spent drawing the changed pages. */
  renderMs: number;
  /** Pages the analysis reused without re-walking them. */
  reusedPages: number;
  /** Checkpoint rungs the engine is holding. */
  checkpointRungs: number;
  /** Bytes of engine state in the checkpoint ladder. */
  checkpointBytes: number;
  /**
   * Checkpoint rung the run resumed from.
   *
   * Anything above zero means the preamble, the document class and everything
   * above the edit were not re-executed. This — not {@link spliceAt} — is what
   * says the run was incremental.
   */
  replayRung: number;
  /** TeX commands executed after the replay origin. */
  replayCommands: number;
  /**
   * Bytes of the previous XDV the replay reused.
   *
   * Only advances at a page boundary, so it stays zero on a one-page document
   * even when the run was fully incremental.
   */
  spliceAt?: number;
  /** Support files resident in the module. */
  supportFiles: number;
  /** Bytes of support files resident in the module. */
  supportBytes: number;
}

/** How far along loading is. */
export interface LoadProgress {
  /** What is happening now. */
  phase:
    | "fetching-module"
    | "instantiating"
    | "fetching-format"
    | "prefetching"
    | "indexing-bundle"
    | "ready";
  /** Bytes retrieved so far in this phase, where that is meaningful. */
  loaded?: number;
  /** Bytes expected in this phase, where that is known. */
  total?: number;
}

/** Settings for {@link TectonicEngine.create}. */
export interface EngineOptions {
  /**
   * Where the compiled `.wasm` lives.
   *
   * Defaults to `tectonic/tectonic_wasm.wasm`, relative to the page.
   */
  moduleUrl?: string;
  /**
   * Where the Worker script lives.
   *
   * Defaults to `worker.js` next to this module, which is what a bundler
   * resolves for you. Override it when you are loading the library from source
   * or from an unusual layout.
   */
  workerUrl?: string | URL;
  /**
   * URL of a Tectonic bundle tar, for anything the support pack does not have.
   *
   * This is what makes arbitrary `\usepackage` work: the pack covers a common
   * core so the first compile never touches the network, and everything else —
   * every package, font and graphic in TeX Live — comes from here on demand.
   *
   * The index beside it (`<url>.index.gz`) is fetched once in the background;
   * individual files are then single HTTP range requests.
   *
   * The host must allow cross-origin range requests. `relay.fullyjustified.net`
   * does not, so the demo proxies it; a real deployment wants its own mirror or
   * proxy. Leave unset to run pack-only.
   */
  bundleUrl?: string;
  /**
   * Base URL of the support pack built by `tools/wasm/build-assets.sh`.
   *
   * Must contain `manifest.json` and a `files/` directory. Defaults to
   * `tectonic/`.
   */
  supportUrl?: string;
  /**
   * Name of the document's main file.
   *
   * It only ever names the file to the engine — nothing is written to disk —
   * but it does decide what the `.log` calls itself. Defaults to `main.tex`.
   */
  primary?: string;
  /**
   * Pull the whole support pack into memory in the background once the engine
   * is ready.
   *
   * This is on by default and worth keeping on: without it, the first compile
   * blocks on a few hundred synchronous requests, one per class and font file,
   * taken from inside the typesetting loop.
   */
  prefetch?: boolean;
  /** Called as loading progresses. */
  onProgress?: (progress: LoadProgress) => void;
  /** Called for each diagnostic the engine emits, as it emits it. */
  onLog?: (line: LogLine) => void;
}

/** What an export produced. */
export interface ExportResult {
  /** The complete PDF. */
  pdf: Uint8Array<ArrayBuffer>;
  /**
   * Frames produced while settling the document before exporting.
   *
   * A viewport-bounded compile stops at the last visible page, and exporting
   * that would give you a PDF missing its tail — so {@link
   * TectonicEngine.exportPdf} finishes the document first. These are the frames
   * that produced, in order, and a viewer that wants to stay in step with the
   * engine should apply them exactly as it applies a compile result. Empty when
   * the document was already settled, which is the usual case.
   */
  tail: CompileResult[];
  /** Milliseconds spent in xdvipdfmx, measured in the Worker. */
  elapsedMs: number;
}

/** A request sent to the Worker. */
export type WorkerRequest =
  | { id: number; kind: "init"; options: SerializableEngineOptions }
  | { id: number; kind: "compile"; text: string; viewport?: [number, number] }
  | { id: number; kind: "continueTail" }
  | { id: number; kind: "export" }
  | { id: number; kind: "cancel" }
  | { id: number; kind: "shutdown" };

/** {@link EngineOptions} minus the callbacks, which cannot cross to a Worker. */
export interface SerializableEngineOptions {
  moduleUrl: string;
  supportUrl: string;
  bundleUrl?: string;
  primary: string;
  prefetch: boolean;
}

/** A message sent back from the Worker. */
export type WorkerResponse =
  | { id: number; kind: "ok" }
  | { id: number; kind: "compiled"; result: CompileResult }
  | { id: number; kind: "exported"; pdf: Uint8Array<ArrayBuffer>; elapsedMs: number }
  | { id: number; kind: "idle" }
  | { id: number; kind: "error"; message: string }
  | { id: 0; kind: "progress"; progress: LoadProgress }
  | { id: 0; kind: "log"; line: LogLine };
