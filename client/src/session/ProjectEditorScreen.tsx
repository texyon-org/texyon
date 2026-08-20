import { getRouteApi, useNavigate } from "@tanstack/react-router";

import { useAccountShow } from "../api/generated";
import { EditorScreen } from "../EditorScreen";
import { useSignOut } from "./useSignOut";

const route = getRouteApi("/projects/$projectId");

export function ProjectEditorScreen() {
  const { project, workspace } = route.useLoaderData();
  const navigate = useNavigate();
  const signOut = useSignOut();
  const account = useAccountShow();

  return (
    <EditorScreen
      // A different project needs a fresh engine.
      key={project.id}
      workspace={workspace}
      primary={project.primary}
      title={project.name}
      projectId={project.id}
      onBack={() => void navigate({ to: "/" })}
      account={account.data?.user ?? null}
      onAccount={() => void navigate({ to: "/account" })}
      onSignOut={signOut}
    />
  );
}
