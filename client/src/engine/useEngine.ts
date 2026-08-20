import type {
  CompileResult,
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
 * Own a resident engine and keep it fed with the workspace.
 *
 * The workspace is the authority on what the files say; this watches it and
 * hands the engine the whole project whenever it moves. Passing everything each
 * time is the cheap path, not a concession — the engine diffs each file against
 * what it last saw, so untouched files cost nothing.
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

    running.current = true;
    setCompiling(true);
    const requestedAt = typedAt.current ?? performance.now();

    try {
      const compiled = await engine.compile(files);
      compiledRevision.current = revision;
      paintingFrom.current = requestedAt;
      setResult(compiled);

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
