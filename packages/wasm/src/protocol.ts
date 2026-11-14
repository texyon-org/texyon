import type { SourceEdit } from "./splice.js";

/** How TeX graded a run. */
export type Outcome = "spotless" | "warnings" | "errors";

/** Severity of a logged line. */
export type LogLevel = "note" | "warning" | "error";

/** Severity of a diagnostic. */
export type Severity = "error" | "warning" | "info";

/** One thing the engine said about one place in the source. */
export interface Diagnostic {
  /** How much it matters. */
  severity: Severity;
  /** The file it belongs to, possibly a class or package, or null outside any file. */
  path: string | null;
  /** One-based source line, when known. */
  line: number | null;
  /** What the engine said. */
  message: string;
  /** TeX's help text, one entry per line. */
  help: string[];
  /** Who issued it, such as `hyperref` or `LaTeX Font`. */
  source: string | null;
}

/** A line the engine logged. */
export interface LogLine {
  /** How bad it is. */
  level: LogLevel;
  /** What it said. */
  message: string;
}

/** A command the document currently has defined. */
export interface EngineCommand {
  /** Without the leading backslash. */
  name: string;
  /** Arguments it takes, an optional one included. */
  arity: number;
  /** Whether the first argument is optional. */
  optional: boolean;
  /** How it was defined. */
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
  /** Target URL, for an external link. */
  uri: string | null;
  /** Named destination, for an internal link. */
  destination: string | null;
}

/** A named spot on a page that a link can reach. */
export interface Destination {
  /** Destination name. */
  name: string;
  /** Big points from the page's left edge. */
  xBp: number;
  /** Big points from the page's top edge. */
  yBp: number;
}

/** A page's links and destinations. */
export interface PageOverlay {
  /** Clickable regions. */
  links: Link[];
  /** Named spots a link can reach. */
  destinations: Destination[];
}

/** One page as SVG. */
export interface RenderedPage {
  /** Zero-based page index. */
  index: number;
  /** Self-contained SVG markup. */
  svg: string;
}

/** One page drawn to pixels by a raster engine. */
export interface PageRaster {
  /** Zero-based page index. */
  index: number;
  /** Pixels per big point. */
  scale: number;
  /** The opaque page. */
  bitmap: ImageBitmap;
}

/** What one compile produced, as a patch against the previous result. */
export interface CompileResult {
  /** Whether the run published a frame. When false, keep the current pages. */
  published: boolean;
  /** SVG for the changed pages, empty for a raster engine. */
  pages: RenderedPage[];
  /** Changed pages near the viewport, drawn at the compile's scale. */
  rasters: PageRaster[];
  /** Indices of the changed pages. */
  changed: number[];
  /** Indices the document no longer has. */
  removed: number[];
  /** Every page's width and height in big points. */
  pageSizes: [number, number][];
  /** Every page's links and destinations. */
  pageOverlays: PageOverlay[];
  /** Pages in the document. */
  pageCount: number;
  /** The full TeX transcript. */
  log: string;
  /** How TeX graded the run. */
  outcome: Outcome;
  /** Why the run was abandoned, if it was. */
  aborted?: string;
  /** Whether a file was missing only because the bundle index had not arrived. A recompile follows. */
  provisional: boolean;
  /** Whether a viewport-bounded compile left pages to typeset. */
  tailPending: boolean;
  /** Status lines from the host and the renderer. */
  messages: string[];
  /** Errors, warnings and info by source line. */
  diagnostics: Diagnostic[];
  /** Timings and incremental statistics. */
  stats: CompileStats;
}

/** What a compile cost and how much it skipped. */
export interface CompileStats {
  /** Request to result, in the Worker. */
  elapsedMs: number;
  /** Time in XeTeX. */
  texMs: number;
  /** Time analyzing pages. */
  analyzeMs: number;
  /** Time drawing pages. */
  renderMs: number;
  /** Checkpoints held for rewinding. */
  checkpointRungs: number;
  /** Bytes those checkpoints hold. */
  checkpointBytes: number;
  /** Checkpoint the run resumed from. Above zero, the preamble was not re-run. */
  replayRung: number;
  /** Commands replayed to get there. */
  replayCommands: number;
  /** Whether the run reused the previous output's prefix. */
  reusedPrefix: boolean;
  /** Pages the engine shipped. */
  shipouts: number;
  /** Pages fingerprinted to find changes. */
  pagesFingerprinted: number;
  /** Pages drawn. */
  pagesRendered: number;
  /** Pages the renderer had to approximate. */
  unsupportedPages: number;
  /** Support files resident in the module. */
  supportFiles: number;
  /** Bytes of those support files. */
  supportBytes: number;
}

/** A project's files by path relative to its root. A changed binary restarts the session. */
export type ProjectFiles = Record<string, string | Uint8Array>;

/** A box a source line produced, in big points. */
export interface SyncRect {
  /** Zero-based page index. */
  pageIndex: number;
  /** Left edge. */
  hBp: number;
  /** Baseline. */
  vBp: number;
  /** Width. */
  widthBp: number;
  /** Height above the baseline. */
  heightBp: number;
  /** Depth below the baseline. */
  depthBp: number;
}

/** Where a source line landed. */
export interface Located {
  /** The box to scroll to. */
  rect: SyncRect;
  /** The boxes to flash on the page of `rect`, best first. */
  boxes: SyncRect[];
}

/** A source line. */
export interface SourceSpot {
  /** Path as given to `compile`. */
  path: string;
  /** One-based line. */
  line: number;
}

/** Loading progress. */
export interface LoadProgress {
  /** Current phase. */
  phase: "fetching-module" | "instantiating" | "fetching-pack" | "indexing-bundle" | "ready";
  /** Bytes loaded in this phase. */
  loaded?: number;
  /** Bytes expected in this phase. */
  total?: number;
}

/** Settings for {@link TexyonEngine.create}. */
export interface EngineOptions {
  /** URL of `texyon_wasm.wasm`. Defaults to the packaged copy. */
  moduleUrl?: string;
  /** URL of the support pack. Defaults to the packaged copy. */
  packUrl?: string;
  /** URL of a `.ttb` bundle for files the pack lacks. The host must allow range requests. */
  bundleUrl?: string;
  /** URL of the Worker script. Defaults to the packaged `worker.js`. */
  workerUrl?: string | URL;
  /** Name of the main file. Defaults to `main.tex`. */
  primary?: string;
  /** Draw pages to bitmaps in the Worker instead of sending SVG. */
  raster?: boolean;
  /** Called as loading progresses. */
  onProgress?: (progress: LoadProgress) => void;
  /** Called for each line the engine logs. */
  onLog?: (line: LogLine) => void;
  /** Called when the bundle index arrived after a provisional compile. */
  onRecompileNeeded?: () => void;
}

/** Progress of an export. */
export type ExportStep = { phase: "pass"; pass: number } | { phase: "writing" };

/** What an export produced. */
export interface ExportResult {
  /** The PDF. */
  pdf: Uint8Array<ArrayBuffer>;
  /** Frames from the full passes, to apply like compile results. */
  tail: CompileResult[];
  /** Whether the passes ran out before the document settled. */
  unsettled: boolean;
  /** Time in xdvipdfmx. */
  elapsedMs: number;
}

/** A source's text at a revision. */
export interface SourceUpsert {
  text: string;
  revision: number;
}

/** A change to a source from revision `base` to `revision`. */
export interface SourceSplice extends SourceEdit {
  base: number;
  revision: number;
}

/** {@link EngineOptions} as sent to the Worker. */
export interface WorkerOptions {
  moduleUrl: string;
  packUrl: string;
  bundleUrl?: string;
  primary: string;
  raster: boolean;
}

/** A request to the Worker. */
export type Request =
  | { kind: "init"; options: WorkerOptions }
  | {
      kind: "compile";
      upserts: Record<string, SourceUpsert>;
      edits: Record<string, SourceSplice>;
      removed: string[];
      viewport?: [number, number];
      scale?: number;
    }
  | { kind: "resources"; upserts: Record<string, Uint8Array>; removed: string[] }
  | { kind: "raster"; pages: number[]; scale: number }
  | { kind: "commands" }
  | { kind: "forwardSearch"; path: string; line: number }
  | { kind: "backwardSearch"; pageIndex: number; hBp: number; vBp: number }
  | { kind: "continueTail" }
  | { kind: "fullPass" }
  | { kind: "export" }
  | { kind: "cancel" }
  | { kind: "shutdown" };

/** A Worker's answer to one request. */
export type Reply =
  | { kind: "ok" }
  | { kind: "idle" }
  | { kind: "compiled"; result: CompileResult }
  | { kind: "passed"; result: CompileResult; changed: boolean }
  | { kind: "exported"; pdf: Uint8Array<ArrayBuffer>; elapsedMs: number }
  | { kind: "located"; located: Located | null; source: SourceSpot | null }
  | { kind: "commands"; commands: EngineCommand[] }
  | { kind: "rastered"; rasters: PageRaster[] }
  | { kind: "error"; message: string };

/** A message the Worker sends unprompted. */
export type WorkerEvent =
  | { kind: "progress"; progress: LoadProgress }
  | { kind: "log"; line: LogLine }
  | { kind: "recompile" };

/** A message to the Worker. */
export type WorkerRequest = Request & { id: number };

/** A message from the Worker. */
export type WorkerResponse = (Reply & { id: number }) | WorkerEvent;
