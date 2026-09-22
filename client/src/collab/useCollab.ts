import { HocuspocusProvider, WebSocketStatus } from "@hocuspocus/provider";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Awareness } from "y-protocols/awareness";
import * as Y from "yjs";

import { readBlob } from "../api/files.ts";
import { collabToken } from "../api/generated";
import { ApiError } from "../api/http";
import type { Role } from "../api/types";
import type { Workspace } from "../workspace/Workspace";
import { CollabBinding } from "./binding.ts";

/** Where this editor stands with the shared document. */
export type CollabStatus = "connecting" | "synced" | "saving" | "offline" | "signed-out" | "removed";

/** Someone else in the project right now. */
export interface Peer {
  clientId: number;
  name: string;
  color: string;
  /** The file they have open. */
  file: string | null;
}

export interface Collab {
  /** Null until the document first synced. Until then the editor shows what the server had and is read-only. */
  binding: CollabBinding | null;
  awareness: Awareness | null;
  status: CollabStatus;
  /** The role the latest token carried, which moves when the owner changes it. */
  role: Role | null;
  peers: Peer[];
  /** Say which file this editor has open, for everyone else's presence list. */
  setOpenFile: (path: string) => void;
  /** Try again after signing back in. */
  reconnect: () => void;
}

const COLORS = ["#e3a6ff", "#7fd1ff", "#ffb86b", "#8be28b", "#ff8fa3", "#f4e06d", "#9fb4ff", "#6fe0c8"];

function colorFor(userId: number): string {
  return COLORS[userId % COLORS.length] ?? COLORS[0]!;
}

function claimsOf(token: string): { uid: number; role: Role; name: string } | null {
  const body = token.split(".")[0];
  if (body === undefined) return null;
  try {
    return JSON.parse(atob(body.replace(/-/g, "+").replace(/_/g, "/"))) as { uid: number; role: Role; name: string };
  } catch {
    return null;
  }
}

function socketUrl(): string {
  return `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host}/collab`;
}

/** The shared document for one project, kept for the editor's lifetime. Null project means the sample. */
export function useCollab(projectId: number | null, workspace: Workspace): Collab | null {
  const [binding, setBinding] = useState<CollabBinding | null>(null);
  const [awareness, setAwareness] = useState<Awareness | null>(null);
  const [status, setStatus] = useState<CollabStatus>("connecting");
  const [role, setRole] = useState<Role | null>(null);
  const [peers, setPeers] = useState<Peer[]>([]);
  const provider = useRef<HocuspocusProvider | null>(null);
  // Set once access is gone, so status events from the dying socket do not paper over it.
  const final = useRef<CollabStatus | null>(null);

  useEffect(() => {
    if (projectId === null) return;
    final.current = null;
    const doc = new Y.Doc();
    const mirror = new CollabBinding(doc, workspace, (digest) => readBlob(projectId, digest));
    let synced = false;
    let unsynced = 0;
    let connected = false;

    const report = (): void => {
      if (final.current !== null) return setStatus(final.current);
      if (!connected) return setStatus(synced ? "offline" : "connecting");
      setStatus(!synced ? "connecting" : unsynced > 0 ? "saving" : "synced");
    };

    const stop = (why: CollabStatus): void => {
      final.current = why;
      report();
      current.disconnect();
    };

    const current: HocuspocusProvider = new HocuspocusProvider({
      url: socketUrl(),
      name: `project.${projectId}`,
      document: doc,
      // Fetched again on every reconnect, which is how a removed member finds out.
      token: async () => {
        try {
          const { token } = await collabToken(projectId);
          const claims = claimsOf(token);
          if (claims !== null) {
            setRole(claims.role);
            current.awareness?.setLocalStateField("user", {
              name: claims.name,
              color: colorFor(claims.uid),
              colorLight: `${colorFor(claims.uid)}33`,
            });
          }
          return token;
        } catch (failure) {
          if (failure instanceof ApiError && failure.isUnauthenticated) stop("signed-out");
          else if (failure instanceof ApiError && (failure.status === 404 || failure.status === 403)) stop("removed");
          throw failure;
        }
      },
      onStatus: ({ status: socket }) => {
        connected = socket === WebSocketStatus.Connected;
        report();
      },
      onSynced: ({ state }) => {
        if (!state) return;
        if (!synced) {
          synced = true;
          mirror.start();
          setBinding(mirror);
        }
        report();
      },
      onUnsyncedChanges: ({ number }) => {
        unsynced = number;
        report();
      },
    });
    provider.current = current;
    setAwareness(current.awareness);

    const onAwareness = (): void => {
      const states = current.awareness?.getStates() ?? new Map();
      const others: Peer[] = [];
      states.forEach((state, clientId) => {
        if (clientId === doc.clientID) return;
        const user = state.user as { name?: string; color?: string } | undefined;
        if (user?.name === undefined) return;
        others.push({
          clientId,
          name: user.name,
          color: user.color ?? COLORS[0]!,
          file: typeof state.file === "string" ? state.file : null,
        });
      });
      setPeers(others);
    };
    current.awareness?.on("change", onAwareness);

    const guard = (event: BeforeUnloadEvent): void => {
      if (unsynced > 0) event.preventDefault();
    };
    window.addEventListener("beforeunload", guard);

    return () => {
      window.removeEventListener("beforeunload", guard);
      current.awareness?.off("change", onAwareness);
      mirror.destroy();
      current.destroy();
      doc.destroy();
      provider.current = null;
      setBinding(null);
      setAwareness(null);
      setPeers([]);
      setStatus("connecting");
    };
  }, [projectId, workspace]);

  const setOpenFile = useCallback((path: string) => {
    provider.current?.awareness?.setLocalStateField("file", path);
  }, []);

  const reconnect = useCallback(() => {
    final.current = null;
    setStatus("connecting");
    void provider.current?.connect();
  }, []);

  if (projectId === null) return null;
  return { binding, awareness, status, role, peers, setOpenFile, reconnect };
}
