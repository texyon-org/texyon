import { HocuspocusProvider } from "@hocuspocus/provider";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import type { Server as HttpServer } from "node:http";
import type { AddressInfo } from "node:net";
import { after, before, test } from "node:test";
import * as Y from "yjs";

import { Laravel } from "./laravel.ts";
import { contentsOf, filesOf } from "./model.ts";
import { deriveSecret } from "./secret.ts";
import { createCollabServer } from "./server.ts";
import { signToken } from "./token.ts";
import type { Role } from "./token.ts";

const secret = deriveSecret("test key");

/** Just enough of Laravel's internal collab endpoints, in memory. */
class FakeLaravel {
  rows = new Map<number, { state: string; epoch: string; files: unknown; binaries: unknown }>();
  seeds = new Map<number, { files: { path: string; text: string }[]; binaries: unknown[] }>();
  stores = 0;
  gone = new Set<number>();
  http: HttpServer;

  constructor() {
    this.http = createServer((request, response) => {
      let body = "";
      request.on("data", (chunk) => (body += chunk));
      request.on("end", () => {
        if (request.headers["x-collab-secret"] !== secret.toString("hex")) return void response.writeHead(403).end();
        const url = new URL(request.url ?? "", "http://x");
        const id = Number(/projects\/(\d+)/.exec(url.pathname)?.[1]);
        if (this.gone.has(id)) return void response.writeHead(404).end();
        const json = (status: number, value: unknown) =>
          response.writeHead(status, { "Content-Type": "application/json" }).end(JSON.stringify(value));
        if (request.method === "GET") {
          const row = this.rows.get(id);
          const seedFrom = this.seeds.get(id) ?? { files: [], binaries: [] };
          return json(200, { state: row?.state ?? null, epoch: row?.epoch ?? null, ...seedFrom });
        }
        const payload = JSON.parse(body) as { state: string; epoch: string; files: unknown; binaries: unknown };
        if (url.searchParams.get("create") === "1" && this.rows.has(id)) return json(409, {});
        this.rows.set(id, payload);
        this.stores++;
        json(200, {});
      });
    });
  }
}

let laravel: FakeLaravel;
let base = "";

before(async () => {
  laravel = new FakeLaravel();
  await new Promise<void>((resolve) => laravel.http.listen(0, "127.0.0.1", resolve));
  base = `http://127.0.0.1:${(laravel.http.address() as AddressInfo).port}`;
});

after(() => {
  laravel.http.closeAllConnections();
  laravel.http.close();
});

async function collab() {
  const server = createCollabServer({
    port: 0,
    secret,
    laravel: new Laravel(base, secret),
    origins: [],
    debounce: 50,
    maxDebounce: 200,
  });
  await server.listen();
  return server;
}

function connect(port: number, project: number, uid: number, role: Role, token?: string) {
  const document = new Y.Doc();
  const failures: string[] = [];
  const provider = new HocuspocusProvider({
    url: `ws://127.0.0.1:${port}/collab`,
    name: `project.${project}`,
    document,
    token: token ?? signToken({ uid, pid: project, role, name: `user ${uid}`, exp: Date.now() / 1000 + 60 }, secret),
    onAuthenticationFailed: ({ reason }) => failures.push(reason),
  });
  const synced = new Promise<void>((resolve) => {
    if (provider.isSynced) resolve();
    provider.on("synced", () => resolve());
  });
  return { document, provider, synced, failures };
}

function textOf(doc: Y.Doc, path: string): Y.Text {
  for (const entry of filesOf(doc).values()) if (entry.get("path") === path) return entry.get("text") as Y.Text;
  throw new Error(`no ${path}`);
}

const settle = (ms = 300) => new Promise((resolve) => setTimeout(resolve, ms));

test("two editors converge on a document seeded from the project's files", async () => {
  laravel.seeds.set(1, { files: [{ path: "main.tex", text: "Hello." }], binaries: [] });
  const server = await collab();
  const a = connect(server.address.port, 1, 1, "owner");
  const b = connect(server.address.port, 1, 2, "editor");
  await Promise.all([a.synced, b.synced]);

  textOf(a.document, "main.tex").insert(0, "A: ");
  textOf(b.document, "main.tex").insert(5, " B");
  await settle();

  assert.equal(textOf(a.document, "main.tex").toString(), textOf(b.document, "main.tex").toString());
  assert.equal(textOf(a.document, "main.tex").toString(), "A: Hello B.");
  assert.ok(laravel.stores >= 2, "the seed and at least one edit were stored");

  a.provider.destroy();
  b.provider.destroy();
  await server.destroy();
});

test("a restart loads the stored document instead of seeding a second copy", async () => {
  laravel.seeds.set(4, { files: [{ path: "main.tex", text: "once" }], binaries: [] });
  const first = await collab();
  const a = connect(first.address.port, 4, 1, "owner");
  await a.synced;
  const before = contentsOf(a.document);
  a.provider.destroy();
  await first.destroy();

  const second = await collab();
  const b = connect(second.address.port, 4, 1, "owner");
  await b.synced;
  assert.deepEqual(contentsOf(b.document), before);
  assert.equal(filesOf(b.document).size, 1);
  b.provider.destroy();
  await second.destroy();
});

test("a viewer's edits are dropped by the server", async () => {
  laravel.seeds.set(2, { files: [{ path: "main.tex", text: "fixed" }], binaries: [] });
  const server = await collab();
  const owner = connect(server.address.port, 2, 1, "owner");
  const viewer = connect(server.address.port, 2, 3, "viewer");
  await Promise.all([owner.synced, viewer.synced]);

  textOf(viewer.document, "main.tex").insert(0, "vandal ");
  await settle();
  assert.equal(textOf(owner.document, "main.tex").toString(), "fixed");

  owner.provider.destroy();
  viewer.provider.destroy();
  await server.destroy();
});

test("a token for another project, or a forged one, is refused", async () => {
  const server = await collab();
  const wrong = connect(server.address.port, 2, 1, "owner",
    signToken({ uid: 1, pid: 99, role: "owner", name: "x", exp: Date.now() / 1000 + 60 }, secret));
  const forged = connect(server.address.port, 2, 1, "owner", "e30.bm9wZQ");
  await settle();
  assert.ok(wrong.failures.length > 0);
  assert.ok(forged.failures.length > 0);
  assert.equal(wrong.provider.isSynced, false);
  wrong.provider.destroy();
  forged.provider.destroy();
  await server.destroy();
});

test("a kick closes one user's connections and nobody else's", async () => {
  const server = await collab();
  const port = server.address.port;
  const keep = connect(port, 2, 1, "owner");
  const gone = connect(port, 2, 5, "editor");
  await Promise.all([keep.synced, gone.synced]);
  let closed = false;
  gone.provider.on("close", () => (closed = true));

  const refused = await fetch(`http://127.0.0.1:${port}/collab/internal/kick`, { method: "POST", body: "{}" });
  assert.equal(refused.status, 403);

  const response = await fetch(`http://127.0.0.1:${port}/collab/internal/kick`, {
    method: "POST",
    headers: { "X-Collab-Secret": secret.toString("hex") },
    body: JSON.stringify({ project: 2, user: 5 }),
  });
  assert.deepEqual(await response.json(), { closed: 1 });
  await settle(100);
  assert.equal(closed, true);

  const health = await fetch(`http://127.0.0.1:${port}/collab/health`);
  assert.equal(await health.text(), "ok");

  keep.provider.destroy();
  gone.provider.destroy();
  await server.destroy();
});

// Hocuspocus keeps the socket of a failed load open past destroy(), hence --test-force-exit.
test("a deleted project cannot be opened", async () => {
  laravel.gone.add(3);
  const server = await collab();
  const client = connect(server.address.port, 3, 1, "owner");
  await settle();
  assert.equal(client.provider.isSynced, false);
  client.provider.destroy();
  await server.destroy();
});
