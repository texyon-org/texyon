import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { logout } from "../api/auth";
import {
  getProjectIndexQueryKey,
  useProjectDestroy,
  useProjectIndex,
  useProjectStore,
} from "../api/generated";
import { ApiError } from "../api/http";
import { ProjectList } from "./ProjectList";

export function ProjectsScreen() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const index = useProjectIndex();
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
      notice={null}
      onOpen={(project) =>
        void navigate({ to: "/projects/$projectId", params: { projectId: project.id } })
      }
      onCreate={async (name) => {
        const { project } = await create.mutateAsync({ data: { name } });
        await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
        void navigate({ to: "/projects/$projectId", params: { projectId: project.id } });
      }}
      onDelete={async (project) => {
        await destroy.mutateAsync({ project: project.id });
        await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
      }}
      onSignOut={() => {
        void (async () => {
          try {
            await logout();
          } catch {
            // Failing still means leaving.
          }
          queryClient.clear();
          void navigate({ to: "/sign-in" });
        })();
      }}
    />
  );
}
