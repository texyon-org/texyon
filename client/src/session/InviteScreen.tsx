import { useQueryClient } from "@tanstack/react-query";
import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { getProjectIndexQueryKey, inviteAccept } from "../api/generated";

const route = getRouteApi("/invite/$token");

/** What an invite link offers, and the one button that takes it. */
export function InviteScreen() {
  const { token } = route.useParams();
  const { invite } = route.useLoaderData();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const join = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const { project } = await inviteAccept(token);
      await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
      void navigate({ to: "/projects/$projectId", params: { projectId: project.id } });
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
      setBusy(false);
    }
  };

  return (
    <div className="gate">
      <div className="card">
        <h1>{invite.project.name}</h1>
        <p className="card-sub">
          {invite.owner.name} invited you to {invite.role === "viewer" ? "view" : "edit"} this project.
        </p>
        {error !== null && <div className="card-error">{error}</div>}
        <button className="btn-primary" disabled={busy} onClick={() => void join()}>
          {busy ? "Joining…" : "Join"}
        </button>
        <Link className="card-link" to="/">
          Back to projects
        </Link>
      </div>
    </div>
  );
}
