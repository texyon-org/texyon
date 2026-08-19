import { useCallback, useEffect, useState } from "react";

import {
  ApiError,
  createProject,
  deleteProject,
  listProjects,
  logout,
} from "./api/client";
import type { Project } from "./api/client";
import { EditorScreen } from "./EditorScreen";
import { openProject } from "./session/load";
import { ProjectList } from "./session/ProjectList";
import { SignIn } from "./session/SignIn";
import { Workspace } from "./workspace/Workspace";
import { SAMPLE } from "./sample";

/**
 * Where in the session's life this tab is.
 *
 * A plain union rather than a router, because none of these places have an
 * address worth sharing — the server decides most transitions anyway, by
 * accepting a cookie or not.
 */
type Stage =
  | { kind: "probing" }
  | { kind: "signed-out"; notice: string | null }
  | { kind: "projects"; list: Project[]; error: string | null }
  | { kind: "opening"; project: Project }
  | { kind: "editing"; project: Project | null; workspace: Workspace };

/** The one question the client cannot answer alone: who is this, and what do they have? */
export function App() {
  const [stage, setStage] = useState<Stage>({ kind: "probing" });

  // Asking for the project list is also how we learn whether we are signed in:
  // there is no whoami endpoint, and this is the answer we would want next
  // anyway.
  const probe = useCallback(() => {
    setStage({ kind: "probing" });
    void (async () => {
      try {
        const list = await listProjects();
        setStage({ kind: "projects", list, error: null });
      } catch (failure) {
        if (failure instanceof ApiError && failure.isUnauthenticated) {
          setStage({ kind: "signed-out", notice: null });
        } else {
          const detail = failure instanceof Error ? failure.message : String(failure);
          setStage({
            kind: "signed-out",
            notice: `The server could not be reached (${detail}). You can still try the sample.`,
          });
        }
      }
    })();
  }, []);

  useEffect(() => {
    probe();
  }, [probe]);

  const open = useCallback((project: Project, listToReturnTo: Project[]) => {
    setStage({ kind: "opening", project });
    void (async () => {
      try {
        const workspace = await openProject(project);
        setStage({ kind: "editing", project, workspace });
      } catch (failure) {
        if (failure instanceof ApiError && failure.isUnauthenticated) {
          setStage({ kind: "signed-out", notice: null });
        } else {
          const detail = failure instanceof Error ? failure.message : String(failure);
          setStage({
            kind: "projects",
            list: listToReturnTo,
            error: `Could not open “${project.name}”: ${detail}`,
          });
        }
      }
    })();
  }, []);

  const signOut = useCallback(() => {
    void (async () => {
      try {
        await logout();
      } catch {
        // A sign-out that fails still means the person wants to be gone; the
        // sign-in screen is right either way.
      }
      setStage({ kind: "signed-out", notice: null });
    })();
  }, []);

  switch (stage.kind) {
    case "probing":
      return <div className="gate">Loading…</div>;

    case "signed-out":
      return (
        <SignIn
          notice={stage.notice}
          onSignedIn={probe}
          onSample={() =>
            setStage({ kind: "editing", project: null, workspace: new Workspace(SAMPLE) })
          }
        />
      );

    case "projects":
      return (
        <ProjectList
          projects={stage.list}
          notice={stage.error}
          onOpen={(project) => open(project, stage.list)}
          onCreate={async (name) => {
            const project = await createProject(name);
            open(project, stage.list);
          }}
          onDelete={async (project) => {
            await deleteProject(project.id);
            setStage({
              kind: "projects",
              list: stage.list.filter((p) => p.id !== project.id),
              error: null,
            });
          }}
          onSignOut={signOut}
        />
      );

    case "opening":
      return <div className="gate">Opening “{stage.project.name}”…</div>;

    case "editing":
      return (
        <EditorScreen
          // A different project is a different world: fresh engine, fresh
          // checkpoints, fresh everything the previous one had warmed up.
          key={stage.project?.id ?? "sample"}
          workspace={stage.workspace}
          primary={stage.project?.primary ?? "main.tex"}
          title={stage.project?.name ?? "Sample"}
          projectId={stage.project?.id ?? null}
          onBack={probe}
        />
      );
  }
}
