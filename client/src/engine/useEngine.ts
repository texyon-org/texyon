import type {
  CompileResult,
  CompileStats,
  EngineCommand,
  LoadProgress,
  LogLine,
  SourceSpot,
  SyncRect,
} from "@tectonic/wasm";
import { TectonicEngine } from "@tectonic/wasm";
import { useCallback, useEffect, useRef, useState } from "react";

import type { Workspace } from "../workspace/Workspace";

export interface EngineState {
  progress: LoadProgress | null;
  startupError: string | null;
  result: CompileResult | null;
  /**
   * What the last keystroke's own compile cost.
   *
   * Not `result.stats`: a tail settle also produces a result, and its cost is
   * background work that would otherwise be read as the price of an edit.
   */
  stats: CompileStats | null;
  compiling: boolean;
  /** Milliseconds from the last keystroke to the pixels. */
  latencyMs: number | null;
  log: LogLine[];
  /**
   * Every command the document has defined.
   *
   * Not refreshed per compile. Walking the hash table costs about as much as a
   * warm compile does and yields around twenty-five thousand entries — nearly
   * two megabytes of JSON to post, parse and re-derive from. Doing that on the
   * keystroke path doubled the cost of an edit for a list that had not
   * changed.
   *
   * It is refreshed when `commandsKey` moves instead: see the parameter.
   */
  commands: readonly EngineCommand[];
}

/**
 * How long the document must hold still before the pages below the fold are
 * finished, at the least. The same base idle the native preview server uses.
 */
const TAIL_IDLE_MS = 200;

/**
 * The idle grows with what settling actually costs, up to this.
 *
 * The engine runs synchronously in its Worker, so a settle cannot be
 * interrupted: a keystroke that arrives mid-settle waits for the whole thing.
 * On a document whose tail is expensive, settling after every 200 ms pause
 * would hand that wait to half the keystrokes. Waiting a couple of tail-costs
 * means the settle only starts once the author has genuinely stopped.
 */
const TAIL_IDLE_MAX_MS = 2000;

/**
 * Own a resident engine and keep it fed with the workspace.
 *
 * The workspace is the authority on what the files say; this watches it and
 * hands the engine the whole project whenever it moves. Passing everything each
 * time is the cheap path, not a concession — the engine diffs each file against
 * what it last saw, so untouched files cost nothing.
 *
 * `viewport` is the visible page range, read at the moment each compile starts.
 * It bounds the foreground run to the pages the user can see: the engine stops
 * at the last visible page and leaves the rest as a tail, which is settled here
 * once the document has been still for {@link TAIL_IDLE_MS}. Without it every
 * keystroke retypesets the whole document below the edit before the user sees
 * anything, which on a long document is most of what an edit costs.
 *
 * `commandsKey` decides when the command list is worth re-reading: whenever it
 * differs from the key the current list was read at, the next compile is
 * followed by a fresh walk. The caller derives it from what actually changes
 * which commands exist — the class and packages the document loads — so typing
 * prose never triggers one. Commands the document declares itself are already
 * offered from the source index, which does keep up with every keystroke.
 */
export function useEngine(
  workspace: Workspace,
  primary: string,
  commandsKey: string,
  viewport: { readonly current: [number, number] },
): EngineState & {
  markPainted: () => void;
  exportPdf: () => Promise<void>;
  forwardSearch: (path: string, line: number) => Promise<SyncRect | null>;
  backwardSearch: (pageIndex: number, hBp: number, vBp: number) => Promise<SourceSpot | null>;
} {
  const [engine, setEngine] = useState<TectonicEngine | null>(null);
  const [progress, setProgress] = useState<LoadProgress | null>(null);
  const [startupError, setStartupError] = useState<string | null>(null);
  const [result, setResult] = useState<CompileResult | null>(null);
  const [stats, setStats] = useState<CompileStats | null>(null);
  const [compiling, setCompiling] = useState(false);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const [log, setLog] = useState<LogLine[]>([]);
  const [commands, setCommands] = useState<readonly EngineCommand[]>([]);

  const typedAt = useRef<number | null>(null);
  const paintingFrom = useRef<number | null>(null);
  const running = useRef(false);
  /** The workspace revision the last compile was of. */
  const compiledRevision = useRef(-1);
  /** The key the command list was read at, or null when it never has been. */
  const commandsAt = useRef<string | null>(null);
  /** Read inside the compile loop, which must not be rebuilt when it moves. */
  const wantedKey = useRef(commandsKey);
  wantedKey.current = commandsKey;
  /** Set below once run() exists, called from the engine's own callback. */
  const recompile = useRef<() => void>(() => {});
  /** Pending idle timer for the document's unsettled tail. */
  const tailTimer = useRef<number | null>(null);
  /** What the last settle cost, which is what the next idle is scaled by. */
  const tailCostMs = useRef(0);
  /** Set below once run() exists; finishes the document below the fold. */
  const settle = useRef<() => void>(() => {});

  useEffect(() => {
    let disposed = false;
    let created: TectonicEngine | null = null;

    void (async () => {
      try {
        const instance = await TectonicEngine.create({
          primary,
          // Root-anchored: the defaults are relative to the page, which breaks
          // on nested routes like /projects/$projectId.
          moduleUrl: "/tectonic/tectonic_wasm.wasm",
          supportUrl: "/tectonic/",
          // `||` rather than `??`: a build arg that was never given arrives as
          // an empty string, and an empty bundle URL means the default one.
          bundleUrl: import.meta.env.VITE_BUNDLE_URL || "/bundle/texlive2026.ttb",
          onProgress: (update) => {
            if (!disposed) setProgress(update.phase === "ready" ? null : update);
          },
          onLog: (line) => {
            if (disposed) return;
            // The transcript is unbounded and mostly noise; keep a tail.
            setLog((prior) => [...prior, line].slice(-200));
          },
          onRecompileNeeded: () => {
            if (!disposed) recompile.current();
          },
        });

        created = instance;
        if (disposed) {
          void instance.close();
          return;
        }
        setEngine(instance);
      } catch (error) {
        if (!disposed) {
          setStartupError(error instanceof Error ? error.message : String(error));
        }
      }
    })();

    return () => {
      disposed = true;
      void created?.close();
    };
  }, [primary]);

  const run = useCallback(async () => {
    if (engine === null || running.current) return;

    const revision = workspace.revision;
    if (revision === compiledRevision.current) return;
    const files = workspace.snapshot();

    // This compile leaves its own tail; a settle scheduled by the last one
    // would only race it for the engine.
    if (tailTimer.current !== null) {
      window.clearTimeout(tailTimer.current);
      tailTimer.current = null;
    }
    running.current = true;
    setCompiling(true);
    const requestedAt = typedAt.current ?? performance.now();

    try {
      const compiled = await engine.compile(files, viewport.current);
      compiledRevision.current = revision;
      paintingFrom.current = requestedAt;
      setResult(compiled);
      setStats(compiled.stats);
      if (compiled.tailPending) {
        const idle = Math.min(TAIL_IDLE_MAX_MS, Math.max(TAIL_IDLE_MS, 2 * tailCostMs.current));
        tailTimer.current = window.setTimeout(() => {
          tailTimer.current = null;
          settle.current();
        }, idle);
      }

      // After the frame, so the preview is never waiting on the command list,
      // and only when the document could plausibly have a different one.
      const wanted = wantedKey.current;
      if (commandsAt.current !== wanted) {
        commandsAt.current = wanted;
        void engine.commands().then(setCommands, () => {
          // Leave the previous list in place, and let the next compile retry.
          commandsAt.current = null;
        });
      }
    } catch (error) {
      setLog((prior) =>
        [
          ...prior,
          { level: "error" as const, message: error instanceof Error ? error.message : String(error) },
        ].slice(-200),
      );
    } finally {
      running.current = false;
      setCompiling(false);

      // The workspace moved while we were busy: go again immediately.
      if (workspace.revision !== compiledRevision.current) void run();
    }
  }, [engine, workspace]);

  recompile.current = () => {
    compiledRevision.current = -1;
    void run();
  };

  settle.current = () => {
    void (async () => {
      if (engine === null || running.current) return;
      // An edit got there first; its own compile reschedules the settle.
      if (workspace.revision !== compiledRevision.current) return;

      running.current = true;
      const startedAt = performance.now();
      try {
        // A continuation can itself stop at a horizon, so settle until the
        // engine says the document is finished.
        for (;;) {
          const frame = await engine.continueTail();
          if (frame === null) break;
          // Background work: its paint must not be measured as latency.
          paintingFrom.current = null;
          setResult(frame);
          if (!frame.tailPending || workspace.revision !== compiledRevision.current) break;
        }
        tailCostMs.current = performance.now() - startedAt;
      } catch (error) {
        setLog((prior) =>
          [
            ...prior,
            { level: "error" as const, message: error instanceof Error ? error.message : String(error) },
          ].slice(-200),
        );
      } finally {
        running.current = false;

        // The workspace moved while the tail was settling: compile it now.
        if (workspace.revision !== compiledRevision.current) void run();
      }
    })();
  };

  // Compile when the workspace moves, and once when the engine arrives.
  //
  // No debounce. There is nothing for one to do here that the engine does not
  // already do better: a compile in flight is left alone, and the run that
  // finishes checks whether the workspace moved meanwhile and goes again — so
  // typing fast coalesces into one follow-up run rather than a queue of them,
  // at whatever rate the engine can actually keep up with. A timer on top of
  // that would only add its own delay to every edit, and on a warm document
  // that delay would dwarf the ten milliseconds it was waiting for.
  useEffect(() => {
    if (engine === null) return;

    const onChange = (): void => {
      typedAt.current = performance.now();
      void run();
    };

    onChange();
    return workspace.subscribe(onChange);
  }, [engine, workspace, run]);

  useEffect(
    () => () => {
      if (tailTimer.current !== null) window.clearTimeout(tailTimer.current);
    },
    [],
  );

  const markPainted = useCallback(() => {
    if (paintingFrom.current === null) return;
    setLatencyMs(performance.now() - paintingFrom.current);
    paintingFrom.current = null;
  }, []);

  const exportPdf = useCallback(async () => {
    if (engine === null) return;

    try {
      const { pdf, tail, elapsedMs } = await engine.exportPdf();
      // Settling the tail produces frames the viewer has not seen. The last is
      // the current state of the document.
      const settled = tail.at(-1);
      if (settled !== undefined) setResult(settled);

      const url = URL.createObjectURL(new Blob([pdf], { type: "application/pdf" }));
      const link = document.createElement("a");
      link.href = url;
      link.download = primary.replace(/\.tex$/i, "") + ".pdf";
      link.click();
      URL.revokeObjectURL(url);

      setLog((prior) =>
        [
          ...prior,
          {
            level: "note" as const,
            message: `exported ${(pdf.length / 1024).toFixed(0)} KB in ${elapsedMs.toFixed(0)} ms`,
          },
        ].slice(-200),
      );
    } catch (error) {
      setLog((prior) =>
        [
          ...prior,
          { level: "error" as const, message: error instanceof Error ? error.message : String(error) },
        ].slice(-200),
      );
    }
  }, [engine, primary]);

  const forwardSearch = useCallback(
    async (path: string, line: number) => (engine === null ? null : engine.forwardSearch(path, line)),
    [engine],
  );

  const backwardSearch = useCallback(
    async (pageIndex: number, hBp: number, vBp: number) =>
      engine === null ? null : engine.backwardSearch(pageIndex, hBp, vBp),
    [engine],
  );

  return {
    progress,
    startupError,
    result,
    stats,
    compiling,
    latencyMs,
    log,
    commands,
    markPainted,
    exportPdf,
    forwardSearch,
    backwardSearch,
  };
}
