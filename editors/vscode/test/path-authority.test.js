"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const { childLogicalPath, isWithin, resolveAuthority, sourceFilePath } = require("../path-authority");

test("accepts normalized TeX children under the canonical primary root", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tectonic-vscode-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const primary = path.join(root, "main.tex");
  const child = path.join(root, "chapters", "one.tex");
  fs.mkdirSync(path.dirname(child));
  fs.writeFileSync(primary, "\\input{chapters/one}");
  fs.writeFileSync(child, "one");
  assert.equal(childLogicalPath(primary, child), "chapters/one.tex");
  assert.equal(childLogicalPath(primary, path.join(root, "image.png")), null);
  assert.equal(childLogicalPath(primary, primary), null);
  assert.equal(sourceFilePath(primary, "chapters/one"), child);
  assert.equal(sourceFilePath(primary, primary), primary);
});

test("resolver authority rejects a lexical child symlinked outside the root", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tectonic-vscode-root-"));
  const outside = fs.mkdtempSync(path.join(os.tmpdir(), "tectonic-vscode-outside-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  t.after(() => fs.rmSync(outside, { recursive: true, force: true }));
  const primary = path.join(root, "main.tex");
  const secret = path.join(outside, "secret.tex");
  const linked = path.join(root, "linked.tex");
  fs.writeFileSync(primary, "test");
  fs.writeFileSync(secret, "secret");
  try {
    fs.symlinkSync(secret, linked);
  } catch (error) {
    if (error.code === "EPERM" || error.code === "EACCES") return t.skip("symlinks unavailable");
    throw error;
  }
  assert.equal(childLogicalPath(primary, linked), null);
  assert.throws(() => sourceFilePath(primary, "linked.tex"), /escapes document root/);
});

test("nearest-existing-ancestor resolution confines unsaved descendants", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tectonic-vscode-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const unresolved = path.join(root, "new", "chapter.tex");
  assert.equal(resolveAuthority(unresolved), unresolved);
  assert.equal(isWithin(resolveAuthority(root), resolveAuthority(unresolved)), true);
});
