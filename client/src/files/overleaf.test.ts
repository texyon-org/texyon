import assert from "node:assert/strict";
import { test } from "node:test";

import { zipSync } from "fflate";

import { projectArchives } from "./overleaf.ts";

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

test("a zip of zips is one project per inner zip", () => {
  const thesis = zipSync({ "main.tex": text("\\documentclass{article}") });
  const paper = zipSync({ "paper.tex": text("\\documentclass{article}") });
  const bulk = zipSync({ "Thesis.zip": thesis, "My Paper.zip": paper, "__MACOSX/._Thesis.zip": text("") });

  const archives = projectArchives("Overleaf Projects.zip", bulk);

  assert.deepEqual(
    archives.map((archive) => archive.name),
    ["My Paper", "Thesis"],
  );
  assert.deepEqual(archives[1]?.bytes, thesis);
});

test("a project zip is a project of its own", () => {
  const project = zipSync({ "main.tex": text("\\documentclass{article}"), "figures/data.zip": zipSync({}) });

  const archives = projectArchives("Thesis.zip", project);

  assert.equal(archives.length, 1);
  assert.equal(archives[0]?.name, "Thesis");
  assert.equal(archives[0]?.bytes, project);
});

test("something that is not a zip throws", () => {
  assert.throws(() => projectArchives("broken.zip", text("not a zip")));
});
