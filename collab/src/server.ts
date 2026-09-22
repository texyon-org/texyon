import { Database } from "@hocuspocus/extension-database";
import { Server } from "@hocuspocus/server";
import type { Connection, Hocuspocus } from "@hocuspocus/server";
import type { IncomingMessage, ServerResponse } from "node:http";
import * as Y from "yjs";

import { Laravel, ProjectGone } from "./laravel.ts";
import { contentsOf, metaOf, seed } from "./model.ts";
import { sameSecret } from "./secret.ts";
import { verifyToken } from "./token.ts";
import type { Claims } from "./token.ts";

export interface CollabOptions {
  port: number;
  address?: string;
  secret: Buffer;
  laravel: Laravel;
  /** Origins a browser may connect from. Empty means any, for development. */
  origins: readonly string[];
  debounce?: number;
  maxDebounce?: number;
  now?: () => number;
}

const ROOM = /^project\.(\d+)$/;

export function projectOf(documentName: string): number | null {
  const match = ROOM.exec(documentName);
  return match === null ? null : Number(match[1]);
}

/** A Hocuspocus server that authenticates with Laravel's tokens and persists through its API. */
export function createCollabServer(options: CollabOptions): Server<Claims> {
  const now = options.now ?? (() => Date.now() / 1000);
  // Stores for one document run one after another, never overlapping.
  const storing = new Map<string, Promise<void>>();

  const server: Server<Claims> = new Server<Claims>({
    port: options.port,
    address: options.address ?? "127.0.0.1",
    quiet: true,
    debounce: options.debounce ?? 2000,
    maxDebounce: options.maxDebounce ?? 10000,

    async onAuthenticate({ token, documentName, requestHeaders, connectionConfig }) {
      const origin = requestHeaders.get("origin");
      if (options.origins.length > 0 && (origin === null || !options.origins.includes(origin))) {
        throw new Error("origin not allowed");
      }
      const claims = verifyToken(token, options.secret, now());
      if (claims === null || projectOf(documentName) !== claims.pid) {
        throw new Error("not authorized for this document");
      }
      // The server drops document updates from a read-only connection, awareness still flows.
      connectionConfig.readOnly = claims.role === "viewer";
      return claims;
    },

    async onRequest({ request, response, instance }) {
      const path = (request.url ?? "").split("?")[0];
      if (path === "/collab/health") {
        response.writeHead(200, { "Content-Type": "text/plain" }).end("ok");
        return Promise.reject();
      }
      if (path === "/collab/internal/kick" && request.method === "POST") {
        await kick(request, response, instance);
        return Promise.reject();
      }
    },

    extensions: [
      new Database({
        async fetch({ documentName }) {
          const projectId = projectOf(documentName);
          if (projectId === null) throw new Error(`not a project document: ${documentName}`);

          const stored = await options.laravel.fetch(projectId);
          if (stored.state !== null) return stored.state;

          // Seed on the server, and store it before anyone syncs, so a restart cannot seed twice.
          const doc = new Y.Doc();
          const epoch = seed(doc, stored);
          const state = Y.encodeStateAsUpdate(doc);
          if (await options.laravel.create(projectId, state, epoch, contentsOf(doc))) return state;
          return (await options.laravel.fetch(projectId)).state;
        },

        async store({ documentName, document, state }) {
          const projectId = projectOf(documentName);
          if (projectId === null) return;
          const previous = storing.get(documentName) ?? Promise.resolve();
          const run = previous
            .catch(() => {})
            .then(() => options.laravel.store(projectId, state, metaOf(document).get("epoch") ?? null, contentsOf(document)))
            .catch((failure: unknown) => {
              if (failure instanceof ProjectGone) {
                server.hocuspocus.closeConnections(documentName);
                return;
              }
              throw failure;
            });
          storing.set(documentName, run);
          try {
            await run;
          } finally {
            if (storing.get(documentName) === run) storing.delete(documentName);
          }
        },
      }),
    ],
  });

  async function kick(request: IncomingMessage, response: ServerResponse, instance: Hocuspocus): Promise<void> {
    const header = request.headers["x-collab-secret"];
    if (!sameSecret(typeof header === "string" ? header : undefined, options.secret)) {
      response.writeHead(403).end();
      return;
    }
    let body: { project?: unknown; user?: unknown };
    try {
      body = JSON.parse(await readBody(request)) as typeof body;
    } catch {
      response.writeHead(400).end();
      return;
    }
    const project = Number.isInteger(body.project) ? (body.project as number) : null;
    const user = Number.isInteger(body.user) ? (body.user as number) : null;

    let closed = 0;
    for (const document of instance.documents.values()) {
      if (project !== null && document.name !== `project.${project}`) continue;
      for (const connection of [...document.connections.keys()] as Connection<Claims>[]) {
        if (user !== null && connection.context?.uid !== user) continue;
        connection.close();
        closed++;
      }
    }
    response.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ closed }));
  }

  return server;
}

function readBody(request: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => {
      body += chunk;
      if (body.length > 4096) request.destroy(new Error("body too large"));
    });
    request.on("end", () => resolve(body));
    request.on("error", reject);
  });
}
