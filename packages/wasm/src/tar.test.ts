import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { gzipSync } from "node:zlib";

import { inflate, TarError, untar } from "./tar.ts";

function archive(files: Record<string, Uint8Array>): Uint8Array {
  const dir = mkdtempSync(join(tmpdir(), "texyon-tar-"));
  try {
    for (const [name, bytes] of Object.entries(files)) writeFileSync(join(dir, name), bytes);
    return new Uint8Array(
      execFileSync("tar", ["--format=ustar", "-cf", "-", "-C", dir, ...Object.keys(files)]),
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const FILES = {
  "manifest.json": new TextEncoder().encode('{"format":"latex.fmt"}'),
  "empty.sty": new Uint8Array(0),
  "block.tfm": new Uint8Array(512).fill(7),
  "odd.pfb": new Uint8Array(513).map((_, i) => i % 251),
};

test("every file comes back with its exact bytes", () => {
  const files = untar(archive(FILES));
  assert.deepEqual([...files.keys()].sort(), Object.keys(FILES).sort());
  for (const [name, bytes] of Object.entries(FILES)) {
    assert.deepEqual([...files.get(name)!], [...bytes], name);
  }
});

test("a web page instead of a pack is refused", () => {
  const page = new Uint8Array(1024);
  page.set(new TextEncoder().encode("<!doctype html><title>Not found</title>"));
  assert.throws(() => untar(page), TarError);
});

test("a cut-off download is refused", () => {
  const whole = archive(FILES);
  assert.throws(() => untar(whole.subarray(0, 1024 + 100)), /truncated|end marker/);
});

test("a gzipped pack is inflated and an already inflated one passes through", async () => {
  const raw = archive(FILES);
  assert.deepEqual([...(await inflate(new Uint8Array(gzipSync(raw))))], [...raw]);
  assert.equal(await inflate(raw), raw);
});
