import { useCallback, useEffect, useRef, useState } from "react";
import {
  TectonicEngine,
  type CompileResult,
  type LoadProgress,
  type LogLine,
} from "@tectonic/wasm";

/**
 * How long to wait after the last keystroke before compiling.
 *
 * A resident edit costs single-digit milliseconds, so this delay is not
 * protecting the engine from work — it only coalesces the several events a
 * single keypress can produce. One frame is enough for that, and anything
 * larger would be the dominant term in the latency the user feels.
 */
const DEBOUNCE_MS = 16;

/** State the UI renders. */
export interface PreviewState {
  /** Null until the engine is ready. */
  engine: TectonicEngine | null;
  /** Loading progress, or null once ready. */
  progress: LoadProgress | null;
  /** Fatal startup error, if the engine never came up. */
  startupError: string | null;
  /** The most recent compile. */
  result: CompileResult | null;
  /** Whether a compile is running right now. */
  compiling: boolean;
  /** Milliseconds from the last keystroke to the pixels. */
  latencyMs: number | null;
  /** Engine diagnostics, newest last. */
  log: LogLine[];
}

/**
 * Own a resident engine and keep it fed with the editor's text.
 *
 * A keystroke starts a debounce; when it fires, the whole buffer goes to the
 * engine, which diffs it against what it last saw. If the user types again
 * while a compile is running, the newer text is sent as soon as the current one
 * returns. Latency is measured from the keystroke that triggered the compile,
 * not from when the compile started, because that is what the user feels.
 */
export function usePreview(source: string): PreviewState & { markPainted: () => void } {
  const [engine, setEngine] = useState<TectonicEngine | null>(null);
  const [progress, setProgress] = useState<LoadProgress | null>(null);
  const [startupError, setStartupError] = useState<string | null>(null);
  const [result, setResult] = useState<CompileResult | null>(null);
  const [compiling, setCompiling] = useState(false);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const [log, setLog] = useState<LogLine[]>([]);

  /** When the keystroke behind the pending compile happened. */
  const typedAt = useRef<number | null>(null);
  /** When the compile whose result we are waiting to paint was requested. */
  const paintingFrom = useRef<number | null>(null);
  const timer = useRef<number | null>(null);
  const running = useRef(false);
  const latest = useRef(source);

  useEffect(() => {
    let disposed = false;
    let created: TectonicEngine | null = null;

    void (async () => {
      try {
        const instance = await TectonicEngine.create({
          // Everything the support pack does not hold comes from here, which is
          // what makes arbitrary \usepackage work. Proxied in vite.config.ts,
          // because the bundle host sends no CORS headers.
          bundleUrl: "/bundle/tlextras-2022.0r0.tar",
          onProgress: (p) => {
            if (!disposed) setProgress(p.phase === "ready" ? null : p);
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
  }, []);

  const run = useCallback(
    async (text: string, requestedAt: number) => {
      if (!engine || running.current) return;

      running.current = true;
      setCompiling(true);

      try {
        const compiled = await engine.compile(text);

        // The result is a patch, so it always supersedes what came before: a
        // run that changed no pages carries an empty page list, and the viewer
        // keeps what it already has.
        paintingFrom.current = requestedAt;
        setResult(compiled);
      } catch (error) {
        setLog((prior) =>
          [
            ...prior,
            {
              level: "error" as const,
              message: error instanceof Error ? error.message : String(error),
            },
          ].slice(-200),
        );
      } finally {
        running.current = false;
        setCompiling(false);

        // Text changed while we were busy: go again immediately.
        if (latest.current !== text) {
          void run(latest.current, typedAt.current ?? performance.now());
        }
      }
    },
    [engine],
  );

  useEffect(() => {
    latest.current = source;
    if (!engine) return;

    typedAt.current = performance.now();

    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      void run(latest.current, typedAt.current ?? performance.now());
    }, DEBOUNCE_MS);

    return () => {
      if (timer.current !== null) clearTimeout(timer.current);
    };
  }, [source, engine, run]);

  /** Called by the preview once the changed pages are actually on screen. */
  const markPainted = useCallback(() => {
    if (paintingFrom.current === null) return;
    setLatencyMs(performance.now() - paintingFrom.current);
    paintingFrom.current = null;
  }, []);

  return {
    engine,
    progress,
    startupError,
    result,
    compiling,
    latencyMs,
    log,
    markPainted,
  };
}
