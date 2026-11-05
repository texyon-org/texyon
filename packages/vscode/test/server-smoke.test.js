"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const { NativeEngine } = require("../engine");

test("the real server opens, draws pixels, finds sources, exports and shuts down", {
  skip: !process.env.TEXYON_PREVIEW_BIN,
  // A cold cache downloads the bundle and builds the format first.
  timeout: 300_000
}, async (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "texyon-smoke-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.writeFileSync(path.join(root, "chapter.tex"), "From disk.\n");
  let stderr = "";
  const engine = new NativeEngine(process.env.TEXYON_PREVIEW_BIN, {
    cwd: root,
    onStderr: (text) => { stderr += text; },
    onExit: () => {}
  });
  t.after(() => engine.process.kill());

  const opened = await engine.request({
    op: "open", format: "latex", primary: "main.tex", root, raster: true, buildDate: 1_700_000_000
  });
  assert.equal(opened.status, "ok", stderr);

  const text = "\\documentclass{article}\\begin{document}Hello preview.\n\\input{chapter}\n\\undefined\n\\end{document}\n";
  const report = await engine.request({
    op: "compile",
    files: { "main.tex": 1 },
    edits: {},
    texts: { "main.tex": text },
    viewport: [0, 0],
    scale: 1
  });
  assert.equal(report.status, "compiled", report.message);
  assert.equal(report.published, true);
  assert.equal(report.pageCount, 1);
  assert.equal(report.rasters.length, 1);
  const pixels = report.rasters[0].data;
  const header = new DataView(pixels.buffer, pixels.byteOffset, 8);
  assert.equal(pixels.byteLength, 8 + header.getUint32(0, true) * header.getUint32(4, true) * 4);
  const error = report.diagnostics.find((found) => found.severity === "error");
  assert.ok(error, "the undefined control sequence is reported");
  assert.equal(error.line, 3);

  const edited = await engine.request({
    op: "compile",
    files: { "main.tex": 2 },
    edits: { "main.tex": { base: 1, from: 0, to: 0, insert: "% note\n" } },
    texts: {},
    viewport: [0, 0],
    scale: 1
  });
  assert.equal(edited.status, "compiled", edited.message);

  const located = await engine.request({ op: "forwardSearch", path: "main.tex", line: 2 });
  assert.equal(located.status, "located");
  assert.equal(located.rect.pageIndex, 0);

  // A full pass typesets the whole document, so export has no tail left to refuse.
  const passed = await engine.request({ op: "fullPass" });
  assert.equal(passed.status, "passed", passed.message);
  assert.equal(passed.tailPending, false);
  assert.match(passed.log, /chapter/);

  const exported = await engine.request({ op: "export", now: 1_700_000_000 });
  assert.equal(exported.status, "exported", exported.message);
  assert.equal(Buffer.from(exported.data.subarray(0, 5)).toString(), "%PDF-");

  await engine.close();
});
