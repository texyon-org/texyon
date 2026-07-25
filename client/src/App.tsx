import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Editor } from "./editor/Editor";
import { useEngine } from "./engine/useEngine";
import { FileTree } from "./files/FileTree";
import { indexProject } from "./project/symbols";
import { Viewer } from "./viewer/Viewer";
import { Workspace } from "./workspace/Workspace";
import { SAMPLE } from "./sample";

/** Render a byte count the way a person would say it. */
function bytes(count: number): string {
  if (count < 1024) return `${count} B`;
  if (count < 1024 * 1024) return `${(count / 1024).toFixed(0)} KB`;
  return `${(count / (1024 * 1024)).toFixed(1)} MB`;
}

const PRIMARY = "main.tex";

export function App() {
  // One workspace for the life of the app: it is the authority on the text, and
  // re-creating it would discard what everything else is watching.
  const [workspace] = useState(() => new Workspace(SAMPLE));
  const [openPath, setOpenPath] = useState(PRIMARY);
  const [revision, setRevision] = useState(0);
  const [showLog, setShowLog] = useState(false);
  const [jumpToLine, setJumpToLine] = useState<number | null>(null);
  const [scrollTo, setScrollTo] = useState<{ pageIndex: number; vBp: number } | null>(null);

  // The workspace is mutable and outside React, so a subscription is what turns
  // its changes into renders.
  useEffect(() => workspace.subscribe((change) => setRevision(change.revision)), [workspace]);

  const {
    progress,
    startupError,
    result,
    compiling,
    latencyMs,
    log,
    markPainted,
    exportPdf,
    forwardSearch,
    backwardSearch,
  } = useEngine(workspace, PRIMARY);

  // Rebuilt when the text changes, which is what makes completion know about
  // the label you wrote a moment ago.
  const symbols = useMemo(() => indexProject(workspace.snapshot()), [workspace, revision]);
  const paths = useMemo(() => workspace.paths(), [workspace, revision]);
  const dirty = useMemo(() => new Set(workspace.dirty()), [workspace, revision]);
  const text = workspace.read(openPath) ?? "";

  const cursorLine = useRef(1);

  const onChange = useCallback(
    (next: string) => {
      workspace.write(openPath, next);
    },
    [workspace, openPath],
  );

  /** Forward search: show where the cursor's line landed on the page. */
  const locate = useCallback(async () => {
    const found = await forwardSearch(openPath, cursorLine.current);
    if (found !== null) setScrollTo({ pageIndex: found.pageIndex, vBp: found.vBp });
  }, [forwardSearch, openPath]);

  /** Backward search: open the source that produced a point on the page. */
  const onBackwardSearch = useCallback(
    (pageIndex: number, hBp: number, vBp: number) => {
      void (async () => {
        const found = await backwardSearch(pageIndex, hBp, vBp);
        if (found === null) return;

        // The engine answers with the path as it was given, so it matches what
        // the workspace calls the file.
        if (workspace.has(found.path)) setOpenPath(found.path);
        // A new object each time, so jumping twice to the same line still moves
        // the cursor.
        setJumpToLine(found.line);
      })();
    },
    [backwardSearch, workspace],
  );

  // Ctrl/Cmd-J, the conventional binding for "show me this in the other pane".
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "j") {
        event.preventDefault();
        void locate();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [locate]);

  const stats = result?.stats;
  const problems = result?.diagnostics.filter((d) => d.severity !== "info") ?? [];

  return (
    <div className="app">
      <header className="bar">
        <div className="title">
          <strong>Tectonic</strong>
          <span className="subtitle">{openPath}</span>
        </div>

        <div className="metrics">
          {latencyMs !== null && (
            <span className="metric primary" title="From the keystroke to the pixels">
              {latencyMs.toFixed(0)} ms
            </span>
          )}
          {stats && (
            <span className="metric" title="TeX, then XDV analysis, then drawing the changed pages">
              {stats.texMs.toFixed(0)} + {stats.analyzeMs.toFixed(0)} + {stats.renderMs.toFixed(0)} ms
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
              title="Resumed from a checkpoint this far in, so the preamble was never re-run"
            >
              rung {stats.replayRung}
            </span>
          )}
          {result && <span className={`metric outcome-${result.outcome}`}>{result.outcome}</span>}
          <span className={`dot ${compiling ? "busy" : "idle"}`} />

          <button onClick={() => void locate()} title="Show this line in the preview (Ctrl-J)">
            Locate
          </button>
          <button disabled={!result} onClick={() => void exportPdf()} title="The preview's own output, as a PDF">
            PDF
          </button>
          <button onClick={() => setShowLog((was) => !was)}>
            {showLog ? "Hide log" : `Log${problems.length > 0 ? ` (${problems.length})` : ""}`}
          </button>
        </div>
      </header>

      {startupError !== null && (
        <div className="banner error">
          <strong>The engine did not start.</strong> {startupError}
          <div className="hint">
            Did you run <code>tools/wasm/build.sh</code> and <code>tools/wasm/build-assets.sh</code>?
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
        <FileTree
          paths={paths}
          open={openPath}
          primary={PRIMARY}
          dirty={dirty}
          onOpen={setOpenPath}
          onCreate={(path) => {
            workspace.create(path, "");
            setOpenPath(path);
          }}
          onRemove={(path) => {
            workspace.remove(path);
            if (path === openPath) setOpenPath(PRIMARY);
          }}
          onRename={(from, to) => {
            if (workspace.rename(from, to) && openPath === from) setOpenPath(to);
          }}
        />

        <section className="pane">
          <Editor
            path={openPath}
            text={text}
            symbols={symbols}
            diagnostics={result?.diagnostics ?? []}
            jumpToLine={jumpToLine}
            onChange={onChange}
            onCursorLine={(line) => {
              cursorLine.current = line;
            }}
          />
        </section>

        <section className="pane">
          <Viewer
            result={result}
            onPainted={markPainted}
            onBackwardSearch={onBackwardSearch}
            scrollTo={scrollTo}
          />
        </section>
      </main>

      {showLog && (
        <aside className="log">
          {result?.aborted !== undefined && <div className="line error">aborted: {result.aborted}</div>}

          {problems.map((problem, index) => (
            <div key={`d${index}`} className={`line ${problem.severity}`}>
              <button
                className="where"
                onClick={() => {
                  if (problem.path !== null && workspace.has(problem.path)) setOpenPath(problem.path);
                  if (problem.line !== null) setJumpToLine(problem.line);
                }}
              >
                {problem.path ?? "?"}
                {problem.line !== null && `:${problem.line}`}
              </button>{" "}
              {problem.message}
            </div>
          ))}

          {problems.length === 0 && log.length === 0 && <div className="line note">No output yet.</div>}

          {log.map((line, index) => (
            <div key={`l${index}`} className={`line ${line.level}`}>
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
