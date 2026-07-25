import { useCallback, useState } from "react";
import { Editor } from "./Editor";
import { Preview } from "./Preview";
import { usePreview } from "./usePreview";
import { SAMPLE } from "./sample";

/** Render a byte count the way a person would say it. */
function bytes(count: number): string {
  if (count < 1024) return `${count} B`;
  if (count < 1024 * 1024) return `${(count / 1024).toFixed(0)} KB`;
  return `${(count / (1024 * 1024)).toFixed(1)} MB`;
}

export function App() {
  const [source, setSource] = useState(SAMPLE);
  const [showLog, setShowLog] = useState(false);
  const { progress, startupError, result, compiling, latencyMs, log, markPainted, exportPdf } =
    usePreview(source);

  const onChange = useCallback((value: string) => setSource(value), []);

  const stats = result?.stats;

  return (
    <div className="app">
      <header className="bar">
        <div className="title">
          <strong>Tectonic</strong>
          <span className="subtitle">LaTeX, compiled to WebAssembly</span>
        </div>

        <div className="metrics">
          {latencyMs !== null && (
            <span className="metric primary" title="From the keystroke to the pixels">
              {latencyMs.toFixed(0)} ms
            </span>
          )}
          {stats && (
            <span
              className="metric"
              title="TeX, then XDV analysis, then drawing the changed pages"
            >
              {stats.texMs.toFixed(0)} + {stats.analyzeMs.toFixed(0)} +{" "}
              {stats.renderMs.toFixed(0)} ms
            </span>
          )}
          {result && (
            <span
              className="metric"
              title="Pages redrawn, out of the whole document. The rest kept their pixels."
            >
              {result.changed.length}/{result.pageCount} redrawn
            </span>
          )}
          {stats && stats.replayRung > 0 && (
            <span
              className="metric"
              title="Resumed from a checkpoint this far into the document, so the preamble was never re-run"
            >
              rung {stats.replayRung}
            </span>
          )}
          {result && (
            <span className={`metric outcome-${result.outcome}`}>{result.outcome}</span>
          )}
          <span className={`dot ${compiling ? "busy" : "idle"}`} />
          <button
            className="log-toggle"
            disabled={!result}
            title="The preview's own output, run through xdvipdfmx"
            onClick={() => void exportPdf()}
          >
            PDF
          </button>
          <button className="log-toggle" onClick={() => setShowLog((v) => !v)}>
            {showLog ? "Hide log" : "Log"}
          </button>
        </div>
      </header>

      {startupError && (
        <div className="banner error">
          <strong>The engine did not start.</strong> {startupError}
          <div className="hint">
            Did you run <code>tools/wasm/build.sh</code> and{" "}
            <code>tools/wasm/build-assets.sh</code>?
          </div>
        </div>
      )}

      {progress && (
        <div className="banner">
          {progress.phase === "fetching-module" && "Fetching the engine…"}
          {progress.phase === "instantiating" && "Starting the engine…"}
          {progress.phase === "fetching-format" && "Loading the LaTeX format…"}
          {progress.phase === "prefetching" &&
            `Caching support files… ${progress.loaded ?? 0}/${progress.total ?? 0}`}
          {progress.phase === "indexing-bundle" && "Indexing the TeX Live bundle…"}
        </div>
      )}

      <main className="panes">
        <section className="pane">
          <Editor initialValue={SAMPLE} onChange={onChange} />
        </section>

        <section className="pane">
          <Preview result={result} onPainted={markPainted} />
        </section>
      </main>

      {showLog && (
        <aside className="log">
          {result?.aborted && <div className="line error">aborted: {result.aborted}</div>}
          {log.length === 0 && <div className="line note">No output yet.</div>}
          {log.map((line, index) => (
            <div key={index} className={`line ${line.level}`}>
              {line.message}
            </div>
          ))}
          {result && (
            <details className="transcript">
              <summary>TeX transcript ({bytes(result.log.length)})</summary>
              <pre>{result.log}</pre>
            </details>
          )}
        </aside>
      )}
    </div>
  );
}
