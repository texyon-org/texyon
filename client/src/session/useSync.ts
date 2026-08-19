/**
 * A `ProjectSync` with a React lifetime.
 *
 * Owns one sync per (workspace, project) pair, surfaces its status as state,
 * and warns before the tab closes on unsaved work. With no project — the
 * sample, open without an account — there is nothing to sync and the status
 * stays null, which is how the editor knows not to show a save indicator.
 */

import { useCallback, useEffect, useRef, useState } from "react";

import { projectFileDestroy, projectFilePut } from "../api/generated";
import type { Workspace } from "../workspace/Workspace";
import { ProjectSync } from "./sync";
import type { SyncStatus } from "./sync";

export function useSync(
  workspace: Workspace,
  projectId: number | null,
): { status: SyncStatus | null; retry: () => void } {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const syncRef = useRef<ProjectSync | null>(null);

  useEffect(() => {
    if (projectId === null) return;

    const sync = new ProjectSync(
      workspace,
      {
        writeFile: (path, text) => projectFilePut(projectId, { text }, { path }),
        deleteFile: (path) => projectFileDestroy(projectId, { path }),
      },
      setStatus,
    );
    syncRef.current = sync;
    setStatus("synced");

    return () => {
      sync.stop();
      syncRef.current = null;
      setStatus(null);
    };
  }, [workspace, projectId]);

  useEffect(() => {
    const guard = (event: BeforeUnloadEvent): void => {
      if (syncRef.current?.pending === true) event.preventDefault();
    };
    window.addEventListener("beforeunload", guard);
    return () => window.removeEventListener("beforeunload", guard);
  }, []);

  const retry = useCallback(() => syncRef.current?.retry(), []);

  return { status, retry };
}
