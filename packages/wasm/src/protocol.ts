/**
 * Types shared between the page and the Worker, and with the Rust module.
 *
 * The Rust side of these lives in `crates/wasm/src/protocol.rs`; the two must
 * agree on field names, which are camelCase on both sides.
 */

import type { SourceEdit } from "./splice.js";

/** How a compile ended, as TeX itself grades it. */
export type Outcome = "spotless" | "warnings" | "errors";

/** Severity of a line the engine logged. */
export type LogLevel = "note" | "warning" | "error";

/** How much a diagnostic matters. */
export type Severity = "error" | "warning" | "info";

/** One thing the engine said about one place in the source. */
export interface Diagnostic {
  /** How much it matters. */
  severity: Severity;
  /**
   * The file it belongs to, as the transcript named it.
   *
   * Usually one of the paths you passed to {@link TexyonEngine.compile}, but
   * it can name a class or package file when the trouble is inside one. Null
   * when the engine spoke outside any file at all.
   */
  path: string | null;
  /** One-based source line, when the engine named one. */
  line: number | null;
  /** What it said. */
  message: string;
  /** TeX's help text, one entry per line of the transcript. */
  help: string[];
  /** Who issued it when it named itself, such as `hyperref` or `LaTeX Font`. */
  source: string | null;
}

/** One diagnostic from the engine. */
export interface LogLine {
  /** How bad it is. */
  level: LogLevel;
  /** What it said. */
  message: string;
}

/** One command the engine currently has defined. */
export interface EngineCommand {
  /** Without the leading backslash. */
  name: string;
  /** Arguments it takes, including an optional one if it has it. */
  arity: number;
  /** Whether the first argument is optional. */
  optional: boolean;
  /** `macro`, `protected` or `builtin`. */
  kind: "macro" | "protected" | "builtin";
}

/** A clickable region of a page, in big points from its top-left corner. */
export interface Link {
  /** Left edge. */
  left: number;
  /** Top edge. */
  top: number;
  /** Right edge. */
  right: number;
  /** Bottom edge. */
  bottom: number;
  /** An absolute URL, for a link that leaves the document. */
  uri: string | null;
  /** A named destination in this document, for one that does not. */
  destination: string | null;
}

/** A named spot on a page that a link can reach. */
export interface Destination {
  /** The name `\label` and its kin produced. */
  name: string;
  /** Horizontal position in big points from the page's left edge. */
  xBp: number;
  /** Vertical position in big points from the page's top edge. */
  yBp: number;
}

/** What one page offers beyond its pixels. */
export interface PageOverlay {
  /** Clickable regions. */
  links: Link[];
  /** Named spots a link can reach. */
  destinations: Destination[];
}

/** One page of the rendered document. */
export interface RenderedPage {
  /** Zero-based page index. */
  index: number;
  /** Self-contained SVG markup. */
  svg: string;
}

/** One page drawn to pixels, for an engine created with {@link EngineOptions.raster}. */
export interface PageRaster {
  /** Zero-based page index. */
  index: number;
  /** Pixels per big point it was drawn at. */
  scale: number;
  /** The page, opaque. */
  bitmap: ImageBitmap;
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
  /**
   * Whether the run published a frame.
   *
   * A cancelled or aborted run publishes nothing: its page fields are empty
   * and mean "keep what you have", not "the document has no pages". The log,
   * diagnostics and {@link aborted} are still meaningful.
   */
  published: boolean;
  /**
   * SVG for the changed pages. Its indices are exactly {@link changed}.
   *
   * Empty for a raster engine, which sends {@link rasters} instead.
   */
  pages: RenderedPage[];
  /**
   * The changed pages around the compile's viewport, drawn at its scale.
   *
   * Only for a raster engine given a viewport and a scale. Any other changed
   * page is stale until {@link TexyonEngine.raster} draws it again.
   */
  rasters: PageRaster[];
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
  /**
   * Every page's links and destinations.
   *
   * Covers the whole document rather than only the changed pages, like {@link
   * pageSizes} — an overlay is small, and one that arrived for only some pages
   * would be an overlay whose links sometimes work.
   */
  pageOverlays: PageOverlay[];
  /** Pages in the document. */
  pageCount: number;
  /** TeX's `.log` transcript, in full. */
  log: string;
  /** How TeX graded the run. */
  outcome: Outcome;
  /** A recoverable fatal-class abort, if the engine hit one. */
  aborted?: string;
  /**
   * Whether a file was ruled missing only because the bundle index had not
   * arrived yet.
   *
   * Such a run is not the document's verdict: a recompile follows as soon as
   * the index lands (or fails), so a viewer that would show its errors should
   * wait for that one instead.
   */
  provisional: boolean;
  /** Whether a viewport-bounded compile left a tail to finish. */
  tailPending: boolean;
  /** Diagnostics gathered during the run. */
  messages: string[];
  /**
   * What the engine said, and where, recovered from the transcript.
   *
   * The natural source for a lint gutter. `info` is chatty — LaTeX narrates its
   * font substitutions — so most callers want the errors and warnings only.
   */
  diagnostics: Diagnostic[];
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
  /** Checkpoint rungs the engine is holding. */
  checkpointRungs: number;
  /** Bytes of engine state in the checkpoint ladder. */
  checkpointBytes: number;
  /**
   * Checkpoint rung the run resumed from.
   *
   * Anything above zero means the preamble, the document class and everything
   * above the edit were not re-executed. This is what says the run was
   * incremental.
   */
  replayRung: number;
  /** TeX commands executed after the replay origin. */
  replayCommands: number;
  /** Whether the run reused a prefix of the previous run's output. */
  reusedPrefix: boolean;
  /** Fresh pages the engine shipped out. */
  shipouts: number;
  /** Pages the analysis fingerprinted. */
  pagesFingerprinted: number;
  /** Pages drawn again, of those fingerprinted. */
  pagesRendered: number;
  /** Pages the renderer had to approximate. */
  unsupportedPages: number;
  /** Support files resident in the module. */
  supportFiles: number;
  /** Bytes of support files resident in the module. */
  supportBytes: number;
}

/**
 * A project's complete current state, by path relative to its root.
 *
 * Binary files reach the engine as resources, a change to one restarts the session.
 */
export type ProjectFiles = Record<string, string | Uint8Array>;

/** A box on the page that a source line produced. */
export interface SyncRect {
  /** Zero-based output page. */
  pageIndex: number;
  /** Left edge in PostScript big points. */
  hBp: number;
  /** Baseline in PostScript big points. */
  vBp: number;
  /** Width in PostScript big points. */
  widthBp: number;
  /** Height above the baseline in PostScript big points. */
  heightBp: number;
  /** Depth below the baseline in PostScript big points. */
  depthBp: number;
}

/** Where a source line landed on the page. */
export interface Located {
  /** The best box, the one to scroll to. */
  rect: SyncRect;
  /**
   * The boxes to flash, all on the page of `rect`, best first.
   *
   * Empty when the line produced nothing and the one it fell back to is spread
   * over the page, like every line of a beamer frame.
   */
  boxes: SyncRect[];
}

/** Which source line produced a point on a page. */
export interface SourceSpot {
  /** Source path, spelled the way you spelled it. */
  path: string;
  /** One-based source line. */
  line: number;
}

/** How far along loading is. */
export interface LoadProgress {
  /** What is happening now. */
  phase:
    | "fetching-module"
    | "instantiating"
    | "fetching-pack"
    | "indexing-bundle"
    | "ready";
  /** Bytes retrieved so far in this phase, where that is meaningful. */
  loaded?: number;
  /** Bytes expected in this phase, where that is known. */
  total?: number;
}

/** Settings for {@link TexyonEngine.create}. */
export interface EngineOptions {
  /**
   * Where the compiled `.wasm` lives, relative to the page.
   *
   * Defaults to the copy shipped in this package, which a bundler copies into
   * your build.
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
   * The host must allow cross-origin range requests, as
   * `https://bundles.texyon.org/texlive2026.ttb` does. Leave unset to run
   * pack-only.
   */
  bundleUrl?: string;
  /**
   * Where the support pack lives, relative to the page: the format dump and a
   * common set of classes, packages and fonts in one archive.
   *
   * Defaults to the copy shipped in this package, which a bundler copies into
   * your build.
   */
  packUrl?: string;
  /**
   * Name of the document's main file.
   *
   * It only ever names the file to the engine — nothing is written to disk —
   * but it does decide what the `.log` calls itself. Defaults to `main.tex`.
   */
  primary?: string;
  /**
   * Draw pages to pixels in the Worker instead of sending SVG.
   *
   * Showing an SVG page costs the main thread a parse and a paint of every
   * glyph, which on a dense page outweighs typesetting it. A raster engine
   * draws only the pages asked for, at the scale asked for.
   */
  raster?: boolean;
  /** Called as loading progresses. */
  onProgress?: (progress: LoadProgress) => void;
  /** Called for each diagnostic the engine emits, as it emits it. */
  onLog?: (line: LogLine) => void;
  /**
   * Called when a compile that already ran may now resolve files it could
   * not, because the bundle index arrived after it started. Compile again to
   * recover.
   */
  onRecompileNeeded?: () => void;
}

/** Where an export is, for a progress indicator. */
export type ExportStep = { phase: "pass"; pass: number } | { phase: "writing" };

/** What an export produced. */
export interface ExportResult {
  /** The complete PDF. */
  pdf: Uint8Array<ArrayBuffer>;
  /**
   * Frames produced by the full passes before exporting.
   *
   * {@link TexyonEngine.exportPdf} typesets the whole document again until
   * what it reads back (the .aux) stops changing, so references and page
   * numbers agree. These are the frames that produced, in order, and a viewer
   * that wants to stay in step with the engine should apply them exactly as it
   * applies a compile result.
   */
  tail: CompileResult[];
  /** Whether the passes ran out before the document stopped changing. */
  unsettled: boolean;
  /** Milliseconds spent in xdvipdfmx, measured in the Worker. */
  elapsedMs: number;
}

/** A source's complete text, at the revision the page gave it. */
export interface SourceUpsert {
  text: string;
  revision: number;
}

/** A change to a source, from the revision `base` to `revision`. */
export interface SourceSplice extends SourceEdit {
  base: number;
  revision: number;
}

/** A request sent to the Worker. */
export type WorkerRequest =
  | { id: number; kind: "init"; options: SerializableEngineOptions }
  | {
      id: number;
      kind: "compile";
      /** Sources sent whole: new ones, and all of them after a failure. */
      upserts: Record<string, SourceUpsert>;
      /** Sources sent as a splice into the revision the Worker holds. */
      edits: Record<string, SourceSplice>;
      /** Sources the project no longer has. */
      removed: string[];
      viewport?: [number, number];
      /** Pixels per big point for the changed pages around `viewport`. */
      scale?: number;
    }
  | { id: number; kind: "raster"; pages: number[]; scale: number }
  | {
      id: number;
      kind: "resources";
      upserts: Record<string, Uint8Array>;
      removed: string[];
    }
  | { id: number; kind: "commands" }
  | { id: number; kind: "forwardSearch"; path: string; line: number }
  | {
      id: number;
      kind: "backwardSearch";
      pageIndex: number;
      hBp: number;
      vBp: number;
    }
  | { id: number; kind: "continueTail" }
  | { id: number; kind: "fullPass" }
  | { id: number; kind: "export" }
  | { id: number; kind: "cancel" }
  | { id: number; kind: "shutdown" };

/** {@link EngineOptions} minus the callbacks, which cannot cross to a Worker. */
export interface SerializableEngineOptions {
  moduleUrl: string;
  packUrl: string;
  bundleUrl?: string;
  primary: string;
  raster: boolean;
}

/** A message sent back from the Worker. */
export type WorkerResponse =
  | { id: number; kind: "ok" }
  | { id: number; kind: "compiled"; result: CompileResult }
  | { id: number; kind: "passed"; result: CompileResult; changed: boolean }
  | { id: number; kind: "exported"; pdf: Uint8Array<ArrayBuffer>; elapsedMs: number }
  | { id: number; kind: "located"; located: Located | null; source: SourceSpot | null }
  | { id: number; kind: "commands"; commands: EngineCommand[] }
  | { id: number; kind: "rastered"; rasters: PageRaster[] }
  | { id: number; kind: "idle" }
  | { id: number; kind: "error"; message: string }
  | { id: 0; kind: "progress"; progress: LoadProgress }
  | { id: 0; kind: "log"; line: LogLine }
  | { id: 0; kind: "recompile" };
