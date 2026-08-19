import { useState } from "react";

import type { Project } from "../api/client";

export interface ProjectListProps {
  projects: Project[];
  /** Something that went wrong on the way here — a project that would not open. */
  notice: string | null;
  onOpen: (project: Project) => void;
  /** Create a project with this name, then open it. */
  onCreate: (name: string) => Promise<void>;
  onDelete: (project: Project) => Promise<void>;
  onSignOut: () => void;
}

/** When it was, said the way a person would scan a list: date, not time-ago. */
function when(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleDateString();
}

/** The projects an account has, and the way to another one. */
export function ProjectList({
  projects,
  notice,
  onOpen,
  onCreate,
  onDelete,
  onSignOut,
}: ProjectListProps) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const recentFirst = [...projects].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));

  const act = (work: () => Promise<void>): void => {
    void (async () => {
      setBusy(true);
      setError(null);
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
    <div className="gate">
      <div className="card card-wide">
        <div className="card-head">
          <h1>Projects</h1>
          <button className="card-link" onClick={onSignOut}>
            Sign out
          </button>
        </div>

        {notice !== null && <div className="card-error">{notice}</div>}
        {error !== null && <div className="card-error">{error}</div>}

        <ul className="project-list">
          {recentFirst.map((project) => (
            <li key={project.id}>
              <button className="project-open" disabled={busy} onClick={() => onOpen(project)}>
                <span className="project-name">{project.name}</span>
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
                ×
              </button>
            </li>
          ))}
          {recentFirst.length === 0 && <li className="project-none">No projects yet.</li>}
        </ul>

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
          <button type="submit" className="card-primary" disabled={busy || draft.trim() === ""}>
            Create
          </button>
        </form>
      </div>
    </div>
  );
}
