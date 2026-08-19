import { getRouteApi, useNavigate } from "@tanstack/react-router";

import { EditorScreen } from "../EditorScreen";

const route = getRouteApi("/projects/$projectId");

export function ProjectEditorScreen() {
  const { project, workspace } = route.useLoaderData();
  const navigate = useNavigate();

  return (
    <EditorScreen
      // A different project needs a fresh engine.
      key={project.id}
      workspace={workspace}
      primary={project.primary}
      title={project.name}
      projectId={project.id}
      onBack={() => void navigate({ to: "/" })}
    />
  );
}
