import assert from "node:assert/strict";
import { test } from "node:test";
import * as Y from "yjs";

import { contentsOf, filesOf, metaOf, resolvePaths, seed } from "./model.ts";

test("a seeded document gives back the files it was seeded with", () => {
  const doc = new Y.Doc();
  const epoch = seed(doc, {
    files: [{ path: "main.tex", text: "\\\\documentclass{article}" }],
    binaries: [{ path: "fig.png", digest: "a".repeat(64), size: 3 }],
  });
  assert.equal(metaOf(doc).get("epoch"), epoch);
  assert.deepEqual(contentsOf(doc), {
    files: [{ path: "main.tex", text: "\\\\documentclass{article}" }],
    binaries: [{ path: "fig.png", digest: "a".repeat(64), size: 3 }],
  });
});

test("a rename made alongside an edit keeps the edit", () => {
  const one = new Y.Doc();
  seed(one, { files: [{ path: "a.tex", text: "hello" }], binaries: [] });
  const two = new Y.Doc();
  Y.applyUpdate(two, Y.encodeStateAsUpdate(one));

  const [id] = [...filesOf(one).keys()];
  filesOf(one).get(id!)!.set("path", "b.tex");
  (filesOf(two).get(id!)!.get("text") as Y.Text).insert(5, " world");

  Y.applyUpdate(one, Y.encodeStateAsUpdate(two));
  Y.applyUpdate(two, Y.encodeStateAsUpdate(one));
  assert.deepEqual(contentsOf(one).files, [{ path: "b.tex", text: "hello world" }]);
  assert.deepEqual(contentsOf(two), contentsOf(one));
});

test("two entries on one path are told apart the same way everywhere", () => {
  const paths = resolvePaths([
    { id: "b", path: "ch/one.tex" },
    { id: "a", path: "ch/one.tex" },
    { id: "c", path: "ch/one.tex" },
    { id: "d", path: "Makefile" },
    { id: "e", path: "Makefile" },
  ]);
  assert.equal(paths.get("a"), "ch/one.tex");
  assert.equal(paths.get("b"), "ch/one (2).tex");
  assert.equal(paths.get("c"), "ch/one (3).tex");
  assert.equal(paths.get("e"), "Makefile (2)");
});

test("a text file and a binary on the same path do not collide", () => {
  const doc = new Y.Doc();
  seed(doc, { files: [{ path: "x", text: "t" }], binaries: [{ path: "x", digest: "b".repeat(64), size: 1 }] });
  const { files, binaries } = contentsOf(doc);
  assert.notEqual(files[0]!.path, binaries[0]!.path);
});
