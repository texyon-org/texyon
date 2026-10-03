import { useRef, useState } from "react";

import type { Account, Project } from "../api/types";
import { CrossIcon, FileIcon } from "../ui/icons";
import { TopBar } from "../ui/TopBar";

export interface ProjectListProps {
  projects: Project[];
  /** The signed-in account, or null while it loads. */
  account: Account | null;
  /** Something that went wrong on the way here, like a project that would not open. */
  notice: string | null;
  onOpen: (project: Project) => void;
  /** Create a project with this name, then open it. */
  onCreate: (name: string) => Promise<void>;
  /** Make projects from zips, reporting progress, and say how it went. */
  onImport: (files: File[], progress: (status: string) => void) => Promise<string>;
  onDelete: (project: Project) => Promise<void>;
  onAccount: () => void;
  onSignOut: () => void;
}

/** When it was, said the way a person would scan a list: date, not time-ago. */
function when(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return `Edited ${date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })}`;
}

/** The projects an account has, and the way to another one. */
export function ProjectList({
  projects,
  account,
  notice,
  onOpen,
  onCreate,
  onImport,
  onDelete,
  onAccount,
  onSignOut,
}: ProjectListProps) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const picker = useRef<HTMLInputElement>(null);

  const recentFirst = [...projects].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  const owned = recentFirst.filter((project) => project.role === "owner");
  const shared = recentFirst.filter((project) => project.role !== "owner");

  const act = (work: () => Promise<void>): void => {
    void (async () => {
      setBusy(true);
      setError(null);
      setStatus(null);
      try {
        await work();
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    })();
  };

  return (
    <div className="screen">
      <TopBar account={account} onBack={null} onAccount={onAccount} onSignOut={onSignOut} />

      <main className="screen-main">
        <div className="screen-column">
          <div className="screen-head">
            <div className="screen-title">
              <h1>Projects</h1>
              <span className="screen-count">{owned.length}</span>
            </div>

            <form
              className="project-new"
              onSubmit={(event) => {
                event.preventDefault();
                const name = draft.trim();
                if (name === "") return;
                act(async () => {
                  await onCreate(name);
                  setDraft("");
                });
              }}
            >
              <input
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                placeholder="A new project's name"
                disabled={busy}
              />
              <button type="submit" className="btn-primary" disabled={busy || draft.trim() === ""}>
                Create
              </button>
              <button
                type="button"
                className="btn-secondary"
                title="Project zips, or Overleaf's download of several projects at once"
                disabled={busy}
                onClick={() => picker.current?.click()}
              >
                Import
              </button>
              <input
                ref={picker}
                type="file"
                accept=".zip,application/zip"
                multiple
                hidden
                onChange={(event) => {
                  const files = [...(event.target.files ?? [])];
                  event.target.value = "";
                  if (files.length === 0) return;
                  act(async () => setStatus(await onImport(files, setStatus)));
                }}
              />
            </form>
          </div>

          {notice !== null && <div className="notice error">{notice}</div>}
          {error !== null && <div className="notice error">{error}</div>}
          {status !== null && <div className={busy ? "notice" : "notice ok"}>{status}</div>}

          <ul className="project-rows">
            {owned.map((project) => (
              <li key={project.id}>
                <button className="project-open" disabled={busy} onClick={() => onOpen(project)}>
                  <FileIcon />
                  <span className="project-info">
                    <span className="project-name">{project.name}</span>
                    <span className="project-primary">{project.primary}</span>
                  </span>
                  <span className="project-when">{when(project.updatedAt)}</span>
                </button>
                <button
                  className="project-delete"
                  title="Delete this project and its files"
                  disabled={busy}
                  onClick={() => {
                    if (window.confirm(`Delete “${project.name}” and its files?`)) {
                      act(() => onDelete(project));
                    }
                  }}
                >
                  <CrossIcon size={14} />
                </button>
              </li>
            ))}
            {owned.length === 0 && <li className="project-none">No projects yet.</li>}
          </ul>

          {shared.length > 0 && (
            <>
              <div className="screen-title">
                <h2>Shared with me</h2>
                <span className="screen-count">{shared.length}</span>
              </div>
              <ul className="project-rows">
                {shared.map((project) => (
                  <li key={project.id}>
                    <button className="project-open" disabled={busy} onClick={() => onOpen(project)}>
                      <FileIcon />
                      <span className="project-info">
                        <span className="project-name">{project.name}</span>
                        <span className="project-primary">
                          {project.owner.name} · {project.role === "viewer" ? "can view" : "can edit"}
                        </span>
                      </span>
                      <span className="project-when">{when(project.updatedAt)}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      </main>
    </div>
  );
}
