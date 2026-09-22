import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import {
  getProjectIndexQueryKey,
  getProjectMemberIndexQueryKey,
  projectInviteDestroy,
  projectInviteStore,
  projectMemberDestroy,
  projectMemberStore,
  projectMemberUpdate,
  useProjectMemberIndex,
} from "../api/generated";
import type { Role } from "../api/types";
import { CrossIcon } from "../ui/icons";

export interface ShareDialogProps {
  projectId: number;
  projectName: string;
  /** Only the owner can change who has access. */
  role: Role;
  /** The signed-in account's email, so a member can find themselves and leave. */
  selfEmail: string | null;
  onClose: () => void;
  /** After the signed-in user left the project. */
  onLeft: () => void;
}

type SharedRole = "editor" | "viewer";

const ROLE_LABEL: Record<Role, string> = { owner: "Owner", editor: "Can edit", viewer: "Can view" };

export function inviteUrl(token: string): string {
  return `${window.location.origin}/invite/${token}`;
}

/** Who a project is shared with, and for the owner the controls to change it. */
export function ShareDialog({ projectId, projectName, role, selfEmail, onClose, onLeft }: ShareDialogProps) {
  const queryClient = useQueryClient();
  const members = useProjectMemberIndex(projectId);
  const [email, setEmail] = useState("");
  const [emailRole, setEmailRole] = useState<SharedRole>("editor");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Tokens exist only in the response that created them, so keep the fresh ones here.
  const [links, setLinks] = useState<Record<number, string>>({});
  const [copied, setCopied] = useState<number | null>(null);
  const owner = role === "owner";

  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const act = (work: () => Promise<void>): void => {
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        await work();
        await queryClient.invalidateQueries({ queryKey: getProjectMemberIndexQueryKey(projectId) });
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    })();
  };

  const copy = (id: number, token: string): void => {
    void navigator.clipboard.writeText(inviteUrl(token)).then(() => setCopied(id));
  };

  const data = members.data;
  const me = data?.members.find((member) => member.email === selfEmail) ?? null;

  return (
    <div className="overlay" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-label={`Share ${projectName}`}>
        <div className="dialog-head">
          <h1>Share “{projectName}”</h1>
          <button className="dialog-close" onClick={onClose} title="Close">
            <CrossIcon size={14} />
          </button>
        </div>

        {error !== null && <div className="card-error">{error}</div>}

        {owner && (
          <form
            className="share-add"
            onSubmit={(event) => {
              event.preventDefault();
              const address = email.trim();
              if (address === "") return;
              act(async () => {
                await projectMemberStore(projectId, { email: address, role: emailRole });
                setEmail("");
              });
            }}
          >
            <input
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="Someone's account email"
              disabled={busy}
            />
            <select value={emailRole} onChange={(event) => setEmailRole(event.target.value as SharedRole)} disabled={busy}>
              <option value="editor">Can edit</option>
              <option value="viewer">Can view</option>
            </select>
            <button type="submit" className="btn-primary" disabled={busy || email.trim() === ""}>
              Share
            </button>
          </form>
        )}

        <ul className="share-people">
          {data && (
            <li>
              <span className="share-who">
                <span className="share-name">{data.owner.name}</span>
                <span className="share-email">{data.owner.email}</span>
              </span>
              <span className="share-role">{ROLE_LABEL.owner}</span>
            </li>
          )}
          {data?.members.map((member) => (
            <li key={member.id}>
              <span className="share-who">
                <span className="share-name">{member.name}</span>
                <span className="share-email">{member.email}</span>
              </span>
              {owner ? (
                <>
                  <select
                    value={member.role}
                    disabled={busy}
                    onChange={(event) =>
                      act(async () => {
                        await projectMemberUpdate(projectId, member.id, { role: event.target.value as SharedRole });
                      })
                    }
                  >
                    <option value="editor">Can edit</option>
                    <option value="viewer">Can view</option>
                  </select>
                  <button
                    className="share-remove"
                    title={`Remove ${member.name}`}
                    disabled={busy}
                    onClick={() => act(() => projectMemberDestroy(projectId, member.id))}
                  >
                    <CrossIcon size={12} />
                  </button>
                </>
              ) : (
                <span className="share-role">{ROLE_LABEL[member.role as Role] ?? member.role}</span>
              )}
            </li>
          ))}
          {members.isPending && <li className="share-none">Loading…</li>}
        </ul>

        {owner && (
          <section className="share-links">
            <div className="share-links-head">
              <h2>Invite links</h2>
              <span className="share-hint">Anyone with an account who opens one joins.</span>
            </div>
            {data?.invites.map((invite) => {
              const token = links[invite.id];
              return (
                <div className="share-link" key={invite.id}>
                  <span className="share-role">{ROLE_LABEL[invite.role as Role] ?? invite.role}</span>
                  {token !== undefined ? (
                    <input className="share-url" readOnly value={inviteUrl(token)} onFocus={(event) => event.target.select()} />
                  ) : (
                    <span className="share-hint">Created {new Date(invite.createdAt).toLocaleDateString()}</span>
                  )}
                  {token !== undefined && (
                    <button disabled={busy} onClick={() => copy(invite.id, token)}>
                      {copied === invite.id ? "Copied" : "Copy"}
                    </button>
                  )}
                  <button
                    disabled={busy}
                    title="Stop this link from working. People who joined keep access."
                    onClick={() => act(() => projectInviteDestroy(projectId, invite.id))}
                  >
                    Revoke
                  </button>
                </div>
              );
            })}
            <div className="share-link-new">
              {(["editor", "viewer"] as const).map((linkRole) => (
                <button
                  key={linkRole}
                  disabled={busy}
                  onClick={() =>
                    act(async () => {
                      const { invite } = await projectInviteStore(projectId, { role: linkRole });
                      setLinks((was) => ({ ...was, [invite.id]: invite.token }));
                      copy(invite.id, invite.token);
                    })
                  }
                >
                  New {linkRole === "editor" ? "edit" : "view"} link
                </button>
              ))}
            </div>
          </section>
        )}

        {!owner && me !== null && (
          <button
            className="share-leave"
            disabled={busy}
            onClick={() => {
              if (!window.confirm(`Leave “${projectName}”? You will need a new invite to get back in.`)) return;
              act(async () => {
                await projectMemberDestroy(projectId, me.id);
                await queryClient.invalidateQueries({ queryKey: getProjectIndexQueryKey() });
                onLeft();
              });
            }}
          >
            Leave this project
          </button>
        )}
      </div>
    </div>
  );
}
