import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import {
  getProjectIndexQueryKey,
  useAccountShow,
  useProjectDestroy,
  useProjectIndex,
  useProjectStore,
} from "../api/generated";
import { importProject } from "../api/files";
import { ApiError } from "../api/http";
import { projectArchives } from "../files/overleaf";
import { ProjectList } from "./ProjectList";
import { useSignOut } from "./useSignOut";

export function ProjectsScreen() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const signOut = useSignOut();

  const index = useProjectIndex();
  const account = useAccountShow();
  const create = useProjectStore();
  const destroy = useProjectDestroy();

  // The session can lapse while this screen sits open.
  useEffect(() => {
    if (index.error instanceof ApiError && index.error.isUnauthenticated) {
      void navigate({ to: "/sign-in" });
    }
  }, [index.error, navigate]);

  return (
    <ProjectList
      projects={index.data?.projects ?? []}
      account={account.data?.user ?? null}
      notice={null}
      onOpen={(project) =>
        void navigate({ to: "/projects/$projectId", params: { projectId: project.id } })
      }
      onCreate={async (name) => {
        const { project } = await create.mutateAsync({ data: { name } });
        await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
        void navigate({ to: "/projects/$projectId", params: { projectId: project.id } });
      }}
      onImport={async (files, progress) => {
        try {
          return await importAll(files, progress);
        } finally {
          await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
        }
      }}
      onDelete={async (project) => {
        await destroy.mutateAsync({ project: project.id });
        await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
      }}
      onAccount={() => void navigate({ to: "/account" })}
      onSignOut={signOut}
    />
  );
}

/** One project per archive, one at a time so a bulk export never sits in memory unpacked whole. */
async function importAll(files: File[], progress: (status: string) => void): Promise<string> {
  let imported = 0;
  let skipped = 0;
  const failures: string[] = [];

  for (const file of files) {
    progress(`Reading ${file.name}`);
    let archives;
    try {
      archives = projectArchives(file.name, new Uint8Array(await file.arrayBuffer()));
    } catch {
      failures.push(`${file.name} could not be read as a zip`);
      continue;
    }

    for (const [at, archive] of archives.entries()) {
      progress(`Importing ${archive.name} (${at + 1} of ${archives.length})`);
      try {
        const result = await importProject(archive.name, archive.bytes);
        imported += 1;
        skipped += result.skipped.length;
      } catch (failure) {
        failures.push(`${archive.name}: ${failure instanceof Error ? failure.message : String(failure)}`);
      }
    }
  }

  const summary =
    `Imported ${imported} ${imported === 1 ? "project" : "projects"}` +
    (skipped > 0 ? `, ${skipped} ${skipped === 1 ? "file" : "files"} skipped` : "") +
    ".";
  if (failures.length > 0) throw new Error(`${summary} Failed: ${failures.join("; ")}`);
  return summary;
}
