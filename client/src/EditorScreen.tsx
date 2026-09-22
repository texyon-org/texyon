import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { CwlPackage } from "./project/cwl";
import { Editor } from "./editor/Editor";
import { useEngine } from "./engine/useEngine";
import { BinaryFileView } from "./files/BinaryFileView";
import { FileTree } from "./files/FileTree";
import { planImport } from "./files/import";
import { entriesFromUploads } from "./files/upload";
import type { UploadFile } from "./files/upload";
import { mergeCommands, mergeEnvironments, mergeKeys, packagesUsed } from "./project/commands";
import { corpusOf, providers } from "./project/corpus";
import { indexProject } from "./project/symbols";
import type { Account, Role } from "./api/types";
import { ShareDialog } from "./session/ShareDialog";
import { SignIn } from "./session/SignIn";
import { useSync } from "./session/useSync";
import { TopBar } from "./ui/TopBar";
import { Viewer } from "./viewer/Viewer";
import type { Workspace } from "./workspace/Workspace";

/** Render a byte count the way a person would say it. */
function bytes(count: number): string {
  if (count < 1024) return `${count} B`;
  if (count < 1024 * 1024) return `${(count / 1024).toFixed(0)} KB`;
  return `${(count / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * How long the set of loaded packages must hold still before it is believed.
 *
 * Long enough to cover typing a package name out, short enough that adding one
 * and looking for its commands does not feel like waiting.
 */
const PACKAGE_SETTLE_MS = 500;

/** What the save indicator says, per state the sync can be in. */
const SYNC_LABEL = {
  synced: "saved",
  dirty: "unsaved",
  saving: "saving…",
  failed: "save failed",
  "signed-out": "signed out",
} as const;

export interface EditorScreenProps {
  /** The authority on the text, for this screen's whole lifetime. */
  workspace: Workspace;
  /** The file the compile starts from. */
  primary: string;
  /** What to call this project in the bar. */
  title: string;
  /** The server-side project being kept up to date, or null for the sample. */
  projectId: number | null;
  onBack: () => void;
  /** The signed-in account for the bar's menu, or null for the sample. */
  account?: Account | null;
  onAccount?: (() => void) | null;
  onSignOut?: () => void;
  /** What the signed-in user may do here. The sample is always the owner's. */
  role?: Role;
  /** After the signed-in user left a project shared with them. */
  onLeft?: () => void;
}

/** The editor proper: files, text, preview — around one project. */
export function EditorScreen({
  workspace,
  primary,
  title,
  projectId,
  onBack,
  account = null,
  onAccount = null,
  onSignOut = () => {},
  role = "owner",
  onLeft = () => {},
}: EditorScreenProps) {
  const readOnly = role === "viewer";
  const [sharing, setSharing] = useState(false);
  const [openPath, setOpenPath] = useState(primary);
  const [revision, setRevision] = useState(0);
  const [showLog, setShowLog] = useState(false);
  const [jumpToLine, setJumpToLine] = useState<number | null>(null);
  const [scrollTo, setScrollTo] = useState<{ pageIndex: number; vBp: number } | null>(null);
  const [importReport, setImportReport] = useState<{
    imported: number;
    replaced: number;
    problems: string[];
  } | null>(null);

  const onUpload = useCallback(
    (uploads: UploadFile[]) => {
      void (async () => {
        const { entries, errors } = await entriesFromUploads(uploads);
        const plan = planImport(entries);

        let replaced = 0;
        for (const file of plan.files) {
          if (workspace.has(file.path)) replaced++;
          workspace.write(file.path, file.content);
        }

        const first = plan.files.find((file) => file.path.endsWith(".tex")) ?? plan.files[0];
        if (first !== undefined) setOpenPath(first.path);

        setImportReport({
          imported: plan.files.length,
          replaced,
          problems: [
            ...errors,
            ...plan.skipped.map((skip) => `${skip.path} (${skip.reason})`),
          ],
        });
      })();
    },
    [workspace],
  );

  // The workspace is mutable and outside React, so a subscription is what turns
  // its changes into renders.
  useEffect(() => workspace.subscribe((change) => setRevision(change.revision)), [workspace]);

  // Nothing a viewer does is saved, the server would refuse it.
  const sync = useSync(workspace, readOnly ? null : projectId);

  const primaryContent = workspace.read(primary);
  const preamble = typeof primaryContent === "string" ? primaryContent.slice(0, 4000) : "";

  // Everything downstream of "what does this document load" hangs off this
  // one string, and it is deliberately not the preamble text: that changes on
  // every keystroke, and re-resolving the corpus and re-walking the engine's
  // hash table for each of them was most of what an edit cost.
  const typedKey = useMemo(() => packagesUsed(preamble).join(" "), [preamble]);

  // Let it settle before acting on it. Half of `\usepackage{booktabs}` reads
  // as a package named `boo`, so typing one out moves the key on every
  // keystroke and each of those would otherwise cost a hash-table walk and a
  // corpus resolution for a package that does not exist.
  //
  // This is a timer on the completion data, not on the edit path — the compile
  // still starts on the keystroke. The cost of being late is that a package
  // you just added takes a moment to offer its commands.
  const [packageKey, setPackageKey] = useState(typedKey);
  useEffect(() => {
    if (typedKey === packageKey) return;
    const timer = setTimeout(() => setPackageKey(typedKey), PACKAGE_SETTLE_MS);
    return () => clearTimeout(timer);
  }, [typedKey, packageKey]);

  // Same list, but with an identity that only moves when the list does, so
  // everything hanging off it can be a plain dependency.
  const packages = useMemo(
    () => (packageKey === "" ? [] : packageKey.split(" ")),
    [packageKey],
  );

  // What the user can currently see, read by each compile as it starts. A ref
  // rather than state: scrolling must not re-render the editor, and the value
  // only matters at the moment a compile is issued.
  const viewport = useRef<[number, number]>([0, 0]);
  const onViewport = useCallback((first: number, last: number) => {
    viewport.current = [first, last];
  }, []);

  const {
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
  } = useEngine(workspace, primary, packageKey, viewport);

  // What the corpus says about the packages this document loads. Fetched from
  // the source text rather than from the engine, because the engine does not
  // record which package defined what — and because this has to work before the
  // first compile.
  const [corpus, setCorpus] = useState<CwlPackage[]>([]);
  useEffect(() => {
    let current = true;
    void corpusOf(packages).then((loaded) => {
      if (current) setCorpus(loaded);
    });
    return () => {
      current = false;
    };
  }, [packages]);

  const merged = useMemo(() => mergeCommands(commands, corpus), [commands, corpus]);
  const environments = useMemo(() => mergeEnvironments(corpus), [corpus]);
  const keys = useMemo(() => mergeKeys(corpus), [corpus]);

  // Rebuilt when the text changes, which is what makes completion know about
  // the label you wrote a moment ago.
  const symbols = useMemo(() => indexProject(workspace.snapshot()), [workspace, revision]);
  const paths = useMemo(() => workspace.paths(), [workspace, revision]);
  const dirty = useMemo(() => new Set(workspace.dirty()), [workspace, revision]);
  const content = workspace.read(openPath) ?? "";
  const text = typeof content === "string" ? content : "";

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

  const problems = result?.diagnostics.filter((d) => d.severity !== "info") ?? [];

  return (
    <div className="app">
      <TopBar
        account={account}
        onBack={onBack}
        onAccount={onAccount}
        onSignOut={onSignOut}
        context={
          <>
            <strong>{title}</strong>
            <span className="subtitle">{openPath}</span>
          </>
        }
        actions={
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

          {sync.status === "failed" ? (
            <button onClick={sync.retry} title="Saving to the server failed, try again">
              retry save
            </button>
          ) : (
            sync.status !== null && (
              <span
                className={`metric sync-${sync.status}`}
                title="Whether the server has what you see"
              >
                {SYNC_LABEL[sync.status]}
              </span>
            )
          )}

          <span className={`dot ${compiling ? "busy" : "idle"}`} />

          {readOnly && (
            <span className="metric" title="The owner shared this project with you read-only">
              view only
            </span>
          )}
          {projectId !== null && (
            <button onClick={() => setSharing(true)} title="Who this project is shared with">
              Share
            </button>
          )}

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
        }
      />

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

      {importReport && (
        <div className={`banner ${importReport.imported === 0 ? "error" : ""}`}>
          <button className="dismiss" title="Dismiss" onClick={() => setImportReport(null)}>
            ✕
          </button>
          {importReport.imported === 0
            ? "Nothing was imported."
            : `Imported ${importReport.imported} file${importReport.imported === 1 ? "" : "s"}` +
              (importReport.replaced > 0 ? `, replacing ${importReport.replaced}.` : ".")}
          {importReport.problems.length > 0 && (
            <div className="hint">Skipped: {importReport.problems.join(", ")}.</div>
          )}
        </div>
      )}

      <main className="panes">
        <FileTree
          paths={paths}
          open={openPath}
          primary={primary}
          dirty={dirty}
          onOpen={setOpenPath}
          onCreate={(path) => {
            workspace.create(path, "");
            setOpenPath(path);
          }}
          onRemove={(path) => {
            workspace.remove(path);
            if (path === openPath) setOpenPath(primary);
          }}
          onRename={(from, to) => {
            if (workspace.rename(from, to) && openPath === from) setOpenPath(to);
          }}
          onUpload={onUpload}
          readOnly={readOnly}
        />

        <section className="pane">
          {typeof content !== "string" ? (
            <BinaryFileView path={openPath} content={content} />
          ) : (
            <Editor
              path={openPath}
              text={text}
              readOnly={readOnly}
              sources={{
                symbols,
                commands: merged,
                environments,
                keys,
                providers: providers(),
                paths,
                openPath,
                addPackage: (name) => {
                  const main = workspace.read(primary);
                  if (typeof main !== "string" || main.includes(`{${name}}`)) return;
                  // after the last \usepackage, else after the \documentclass
                  const anchor = /^.*\\(?:usepackage|documentclass)\b.*$/gm;
                  let last: RegExpExecArray | null = null;
                  for (let m = anchor.exec(main); m !== null; m = anchor.exec(main)) last = m;
                  if (last === null) return;
                  const at = last.index + last[0].length;
                  workspace.write(
                    primary,
                    `${main.slice(0, at)}\n\\usepackage{${name}}${main.slice(at)}`,
                  );
                },
              }}
              diagnostics={result?.diagnostics ?? []}
              jumpToLine={jumpToLine}
              onChange={onChange}
              onCursorLine={(line) => {
                cursorLine.current = line;
              }}
            />
          )}
        </section>

        <section className="pane">
          <Viewer
            result={result}
            onPainted={markPainted}
            onBackwardSearch={onBackwardSearch}
            scrollTo={scrollTo}
            onViewport={onViewport}
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

      {sharing && projectId !== null && (
        <ShareDialog
          projectId={projectId}
          projectName={title}
          role={role}
          selfEmail={account?.email ?? null}
          onClose={() => setSharing(false)}
          onLeft={onLeft}
        />
      )}

      {/* The session lapsed under an open editor. Sign back in over it, so the
          workspace — which holds the unsaved text — is never torn down. */}
      {sync.status === "signed-out" && (
        <div className="overlay">
          <SignIn
            variant="card"
            notice="Your session expired. Sign back in and your changes will be saved. Closing this tab would lose them."
            onSignedIn={sync.retry}
            onSample={null}
          />
        </div>
      )}
    </div>
  );
}
