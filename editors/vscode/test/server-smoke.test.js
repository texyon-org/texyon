"use strict";

const assert = require("node:assert/strict");
const cp = require("node:child_process");
const readline = require("node:readline");
const test = require("node:test");

test("real server orders ready, initialize, update, and shutdown", {
  skip: !process.env.TECTONIC_PREVIEW_BIN,
  timeout: 30_000
}, async () => {
  const child = cp.spawn(process.env.TECTONIC_PREVIEW_BIN, ["-X", "preview", "--format", "latex"], {
    cwd: "/tmp",
    stdio: ["pipe", "pipe", "pipe"]
  });
  let stderr = "";
  child.stderr.on("data", (data) => { stderr += data.toString(); });
  const lines = readline.createInterface({ input: child.stdout });
  const events = [];
  const completed = new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0 && events.some((event) => event.type === "update")) resolve();
      else reject(new Error(`preview exited ${code}: ${stderr}`));
    });
    lines.on("line", (line) => {
      const event = JSON.parse(line);
      events.push(event);
      if (event.type === "ready") {
        child.stdin.write(`${JSON.stringify({
          type: "initialize",
          protocol_version: event.protocol_version,
          primary_path: "/tmp/tectonic-vscode-smoke.tex",
          primary_text: "\\documentclass{article}\\begin{document}Hello preview\\end{document}",
          children: []
        })}\n`);
      } else if (event.type === "update" && event.phase !== "resync") {
        child.stdin.write(`${JSON.stringify({ type: "resync" })}\n`);
      } else if (event.type === "update") {
        child.stdin.end(`${JSON.stringify({ type: "shutdown" })}\n`);
      } else if (event.type === "fatal") {
        reject(new Error(event.reason));
      }
    });
  });
  await completed;
  assert.equal(events[0].type, "ready");
  const update = events.find((event) => event.type === "update");
  assert.equal(update.frame_id, "1");
  assert.equal(update.source_revision, "0");
  assert.ok(update.pages_svg.length > 0);
  assert.equal(update.pages_svg.length, update.changed.length);
  const resync = events.find((event) => event.type === "update" && event.phase === "resync");
  assert.equal(resync.source_revision, update.source_revision);
  assert.equal(resync.changed.length, resync.page_count);
});
