import assert from "node:assert/strict";
import { test } from "node:test";
import * as Y from "yjs";

import { Workspace } from "../workspace/Workspace.ts";
import { CollabBinding } from "./binding.ts";
import { seed } from "./model.ts";

/** Two peers on one document, synced by hand the way a provider would. */
function pair(files: { path: string; text: string }[] = [{ path: "main.tex", text: "Hello." }]) {
  const origin = new Y.Doc();
  seed(origin, { files, binaries: [] });
  const peers = [0, 1].map(() => {
    const doc = new Y.Doc();
    Y.applyUpdate(doc, Y.encodeStateAsUpdate(origin));
    const workspace = new Workspace({});
    const binding = new CollabBinding(doc, workspace, async (digest) => new TextEncoder().encode(`bytes of ${digest}`));
    binding.start();
    return { doc, workspace, binding };
  });
  const sync = () => {
    const [a, b] = peers as [(typeof peers)[0], (typeof peers)[0]];
    Y.applyUpdate(b.doc, Y.encodeStateAsUpdate(a.doc));
    Y.applyUpdate(a.doc, Y.encodeStateAsUpdate(b.doc));
  };
  return { a: peers[0]!, b: peers[1]!, sync };
}

test("the workspace mirrors the document, and edits from either side reach both", () => {
  const { a, b, sync } = pair();
  assert.equal(a.workspace.read("main.tex"), "Hello.");

  a.binding.textAt("main.tex")!.insert(0, "A: ");
  b.binding.textAt("main.tex")!.insert(5, " B");
  sync();

  assert.equal(a.workspace.read("main.tex"), "A: Hello B.");
  assert.equal(b.workspace.read("main.tex"), "A: Hello B.");
  assert.deepEqual(a.workspace.dirty(), [], "the server has it once the document does");
});

test("a rename on one side while the other types keeps the typing", () => {
  const { a, b, sync } = pair();
  const moves: unknown[] = [];
  b.binding.onMoves((m) => moves.push(m));

  assert.equal(a.binding.rename("main.tex", "thesis.tex"), true);
  b.binding.textAt("main.tex")!.insert(6, " More.");
  sync();

  assert.equal(a.workspace.read("thesis.tex"), "Hello. More.");
  assert.equal(b.workspace.read("thesis.tex"), "Hello. More.");
  assert.equal(b.workspace.has("main.tex"), false);
  assert.deepEqual(moves, [{ renamed: new Map([["main.tex", "thesis.tex"]]), removed: [] }]);
});

test("files created on both sides under one name end up apart, the same way on both", () => {
  const { a, b, sync } = pair();
  a.binding.create("ch.tex", "from a");
  b.binding.create("ch.tex", "from b");
  sync();

  assert.deepEqual(a.workspace.paths(), b.workspace.paths());
  assert.deepEqual(a.workspace.paths(), ["ch (2).tex", "ch.tex", "main.tex"]);
  assert.equal(a.workspace.read("ch.tex"), b.workspace.read("ch.tex"));
});

test("removing a file removes it everywhere and says so", () => {
  const { a, b, sync } = pair([
    { path: "main.tex", text: "m" },
    { path: "old.tex", text: "o" },
  ]);
  const moves: unknown[] = [];
  b.binding.onMoves((m) => moves.push(m));

  a.binding.remove("old.tex");
  sync();

  assert.deepEqual(b.workspace.paths(), ["main.tex"]);
  assert.deepEqual(moves, [{ renamed: new Map(), removed: ["old.tex"] }]);
});

test("writing text replaces a file wholesale, and creates it when missing", () => {
  const { a, b, sync } = pair();
  a.binding.writeText("main.tex", "Rewritten.");
  a.binding.writeText("new.tex", "fresh");
  sync();
  assert.equal(b.workspace.read("main.tex"), "Rewritten.");
  assert.equal(b.workspace.read("new.tex"), "fresh");
});

test("a binary is registered by digest, and the other side fetches its bytes", async () => {
  const { a, b, sync } = pair();
  const bytes = new Uint8Array([1, 2, 3]);
  a.binding.setBinary("fig.png", { digest: "d1", size: 3 }, bytes);
  sync();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(a.workspace.read("fig.png"), bytes);
  assert.deepEqual(b.workspace.read("fig.png"), new TextEncoder().encode("bytes of d1"));

  a.binding.writeText("fig.png", "now text");
  sync();
  assert.equal(b.workspace.read("fig.png"), "now text");
});
