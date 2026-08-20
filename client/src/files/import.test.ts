import assert from "node:assert/strict";
import { test } from "node:test";

import { zipSync } from "fflate";

import {
  decodeText,
  expandZip,
  isJunk,
  normalizePath,
  planImport,
  stripCommonRoot,
} from "./import.ts";
import type { ImportEntry } from "./import.ts";

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

test("normalizePath mirrors what the server accepts", () => {
  assert.equal(normalizePath("chapters/one.tex"), "chapters/one.tex");
  assert.equal(normalizePath("./a//b/./c.tex"), "a/b/c.tex");
  assert.equal(normalizePath("a\\b\\c.tex"), "a/b/c.tex");
  assert.equal(normalizePath(""), null);
  assert.equal(normalizePath("../escape.tex"), null);
  assert.equal(normalizePath("a/../b.tex"), null);
  assert.equal(normalizePath("nul\0.tex"), null);
  assert.equal(normalizePath("."), null);
  assert.equal(normalizePath(`${"x".repeat(513)}.tex`), null);
});

test("junk that zips and folders carry is recognized", () => {
  assert.ok(isJunk("__MACOSX/main.tex"));
  assert.ok(isJunk(".DS_Store"));
  assert.ok(isJunk("figures/.DS_Store"));
  assert.ok(isJunk("figures/._plot.pdf"));
  assert.ok(isJunk("Thumbs.db"));
  assert.ok(isJunk("main.tex:Zone.Identifier"));
  assert.ok(!isJunk("chapters/one.tex"));
});

test("decodeText takes UTF-8 and refuses binary", () => {
  assert.equal(decodeText(text("héllo")), "héllo");
  assert.equal(decodeText(new Uint8Array([0x89, 0x50, 0x4e, 0x47])), null);
  assert.equal(decodeText(new Uint8Array([0x61, 0x00, 0x62])), null);
});

test("a lone wrapper directory is stripped, a shared prefix among many is not", () => {
  const wrapped: ImportEntry[] = [
    { path: "thesis/main.tex", bytes: text("a") },
    { path: "thesis/chapters/one.tex", bytes: text("b") },
  ];
  assert.deepEqual(
    stripCommonRoot(wrapped).map((entry) => entry.path),
    ["main.tex", "chapters/one.tex"],
  );

  const mixed: ImportEntry[] = [
    { path: "thesis/main.tex", bytes: text("a") },
    { path: "refs.bib", bytes: text("b") },
  ];
  assert.deepEqual(
    stripCommonRoot(mixed).map((entry) => entry.path),
    ["thesis/main.tex", "refs.bib"],
  );
});

test("planImport keeps text as text and images as bytes, skips the rest with reasons", () => {
  const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
  const plan = planImport([
    { path: "main.tex", bytes: text("\\documentclass{article}") },
    { path: "figures\\plot.png", bytes: png },
    { path: "../escape.tex", bytes: text("nope") },
    { path: ".DS_Store", bytes: new Uint8Array([0]) },
    { path: "big.txt", bytes: new Uint8Array(32 * 1024 * 1024 + 1) },
  ]);

  assert.deepEqual(plan.files, [
    { path: "main.tex", content: "\\documentclass{article}" },
    { path: "figures/plot.png", content: png },
  ]);
  assert.deepEqual(plan.skipped, [
    { path: "../escape.tex", reason: "invalid path" },
    { path: "big.txt", reason: "too large" },
  ]);
});

test("the last entry wins when an upload names a path twice", () => {
  const plan = planImport([
    { path: "main.tex", bytes: text("old") },
    { path: "./main.tex", bytes: text("new") },
  ]);
  assert.deepEqual(plan.files, [{ path: "main.tex", content: "new" }]);
});

test("expandZip unwraps a zipped folder and drops its litter", () => {
  const zipped = zipSync({
    "thesis/main.tex": text("\\documentclass{article}"),
    "thesis/chapters/one.tex": text("one"),
    "thesis/.DS_Store": new Uint8Array([0]),
    "__MACOSX/thesis/._main.tex": new Uint8Array([0]),
  });

  const entries = expandZip(zipped);
  assert.deepEqual(
    entries.map((entry) => entry.path).sort(),
    ["chapters/one.tex", "main.tex"],
  );
  assert.equal(decodeText(entries.find((entry) => entry.path === "main.tex")!.bytes), "\\documentclass{article}");
});

test("expandZip refuses what is not a zip", () => {
  assert.throws(() => expandZip(text("just some text")));
});
