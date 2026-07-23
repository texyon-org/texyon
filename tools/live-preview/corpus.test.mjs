import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  canonicalBelief,
  discoverInputGraph,
  discoverInputs,
  fatalErrorForReason,
  instrumentSource,
  loadDocument,
  mutationCandidates,
  mutateSource,
  parseManifest,
  UnsupportedCorpusError
} from "./corpus.mjs";

test("manifest parsing and source loading stay root-confined", (context) => {
  const tempRoot = process.platform === "win32" ? os.tmpdir() : "/tmp";
  const temp = fs.mkdtempSync(path.join(tempRoot, "tectonic-preview-corpus-"));
  context.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  fs.mkdirSync(path.join(temp, "project", "paper", "nested"), { recursive: true });
  fs.writeFileSync(
    path.join(temp, "project", "paper", "main.tex"),
    "\\input nested/child\n"
  );
  fs.writeFileSync(path.join(temp, "project", "paper", "nested", "child.tex"), "child\n");
  fs.writeFileSync(path.join(temp, "project", "outside.tex"), "outside\n");
  const manifest = path.join(temp, "manifest.tsv");
  const [document] = parseManifest(`fixture\t.\tproject/paper/main.tex\n`, manifest);
  const loaded = loadDocument(document);

  assert.equal(loaded.root, fs.realpathSync(path.join(temp, "project", "paper")));
  assert.equal(loaded.primary, "main.tex");
  assert.deepEqual([...loaded.files.keys()], ["main.tex", "nested/child.tex"]);
});

test("source loading reports the production per-file limit as unsupported", (context) => {
  const tempRoot = process.platform === "win32" ? os.tmpdir() : "/tmp";
  const temp = fs.mkdtempSync(path.join(tempRoot, "tectonic-preview-source-limit-"));
  context.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const primary = path.join(temp, "main.tex");
  fs.writeFileSync(primary, Buffer.alloc(16 * 1024 * 1024 + 1, 0x61));

  assert.throws(
    () => loadDocument({ id: "large-source", root: temp, primary: "main.tex" }),
    (error) =>
      error instanceof UnsupportedCorpusError &&
      error.reasonCode === "source-file-size-limit"
  );
});

test("nested input discovery follows both root-relative and local paths", () => {
  const files = new Map([
    ["main.tex", "% \\\\input{ignored}\n\\input{chapters/one}\n"],
    ["chapters/one.tex", "\\input local\n"],
    ["chapters/local.tex", "\\input{deep/leaf}\n"],
    ["chapters/deep/leaf.tex", "done\n"],
    ["ignored.tex", "not actually consumed\n"]
  ]);
  assert.deepEqual(discoverInputs("main.tex", files), [
    "main.tex",
    "chapters/one.tex",
    "chapters/local.tex",
    "chapters/deep/leaf.tex"
  ]);
  assert.deepEqual(discoverInputGraph("main.tex", files), [
    { source: "main.tex", depth: 0 },
    { source: "chapters/one.tex", depth: 1 },
    { source: "chapters/local.tex", depth: 2 },
    { source: "chapters/deep/leaf.tex", depth: 3 }
  ]);
});

test("instrumentation preserves a TeX format directive and supports same-size variants", () => {
  const first = instrumentSource("%&latex\nbody\n", "LPSTRESS-0000-0000");
  const second = instrumentSource("%&latex\nbody\n", "LPSTRESS-0000-0001");
  assert.ok(first.startsWith("%&latex\n\\message{"));
  assert.equal(Buffer.byteLength(first), Buffer.byteLength(second));
});

test("rendered mutations target safe prose near the source tail", () => {
  const source = [
    "\\documentclass{article}",
    "\\begin{document}",
    "Early ordinary prose is available here.",
    "\\section{Ignored control line}",
    "Late ordinary prose should be selected here.",
    "\\end{document}",
    ""
  ].join("\n");
  const first = mutateSource(source, "LPSTRESS-0000-0000");
  const second = mutateSource(source, "LPSTRESS-0000-0001");

  assert.equal(first.kind, "rendered-text");
  assert.match(first.text, /Late ordinary prose LPSTRESS-0000-0000 be/u);
  assert.equal(Buffer.byteLength(first.text), Buffer.byteLength(second.text));
  assert.ok(first.offset > source.length / 2);
});

test("mutation candidates provide bounded prose retries and an instrumentation fallback", () => {
  const source = Array.from(
    { length: 20 },
    (_, index) => `Ordinary prose candidate number ${index} remains editable.`
  ).join("\n");
  const first = mutationCandidates(source, "LPSTRESS-0000-0000");
  const second = mutationCandidates(source, "LPSTRESS-0000-0001");

  assert.equal(first.length, 9);
  assert.equal(first.at(-1).kind, "instrumentation");
  assert.equal(first[0].kind, "rendered-text");
  assert.equal(first[0].offset, second[0].offset);
  assert.equal(Buffer.byteLength(first[0].text), Buffer.byteLength(second[0].text));
});

test("only the explicit publication cap is classified as unsupported", () => {
  const limited = fatalErrorForReason(
    "preview worker failed: encoded frame has 91320368 bytes, exceeding 67108864"
  );
  const crash = fatalErrorForReason("preview worker failed unexpectedly");

  assert.ok(limited instanceof UnsupportedCorpusError);
  assert.equal(limited.reasonCode, "frame-size-limit");
  assert.ok(!(crash instanceof UnsupportedCorpusError));
});

test("canonical belief excludes operation-local counters and metrics", () => {
  const base = {
    page_count: 1,
    changed: [0],
    removed: [],
    pages_svg: [{ page_index: 0, svg: "<svg/>" }],
    page_sizes: [{ width_bp: 10, height_bp: 20 }],
    unsupported_pages: []
  };
  assert.deepEqual(
    canonicalBelief({ ...base, frame_id: "9", metrics: { engine_ms: 1 } }),
    canonicalBelief({ ...base, frame_id: "42", metrics: { engine_ms: 999 } })
  );
});
