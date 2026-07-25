import assert from "node:assert/strict";
import { test } from "node:test";

import { indexProject } from "./symbols.ts";

test("labels are found across every file in the project", () => {
  const index = indexProject({
    "main.tex": "\\section{One}\\label{sec:one}\n\\input{chapters/two}\n",
    "chapters/two.tex": "\\section{Two}\n\\label{sec:two}\n",
  });

  assert.deepEqual(
    index.labels.map((l) => `${l.name}@${l.path}:${l.line}`),
    ["sec:one@main.tex:1", "sec:two@chapters/two.tex:2"],
  );
});

test("a commented-out declaration is not offered", () => {
  // Suggesting something the author deliberately switched off is worse than
  // suggesting nothing.
  const index = indexProject({
    "main.tex": "% \\label{sec:disabled}\n\\label{sec:live}\ntext % \\label{sec:trailing}\n",
  });

  assert.deepEqual(index.labels.map((l) => l.name), ["sec:live"]);
});

test("an escaped percent does not start a comment", () => {
  // `50\% \label{...}` is a label, not a comment.
  const index = indexProject({ "main.tex": "50\\% off \\label{sec:sale}\n" });

  assert.deepEqual(index.labels.map((l) => l.name), ["sec:sale"]);
});

test("custom commands are found with the argument count they declare", () => {
  const index = indexProject({
    "main.tex": [
      "\\newcommand{\\vect}[1]{\\mathbf{#1}}",
      "\\renewcommand\\foo{bar}",
      "\\newcommand*{\\scaled}[2]{#1#2}",
      "\\DeclareMathOperator{\\argmax}{arg\\,max}",
    ].join("\n"),
  });

  assert.deepEqual(
    index.commands.map((c) => `${c.name}/${c.arity ?? "-"}`),
    ["argmax/-", "foo/-", "scaled/2", "vect/1"],
  );
});

test("environments are found separately from commands", () => {
  const index = indexProject({
    "main.tex": "\\newenvironment{aside}[1]{\\begin{quote}}{\\end{quote}}\n",
  });

  assert.deepEqual(index.environments.map((e) => `${e.name}/${e.arity ?? "-"}`), ["aside/1"]);
  assert.deepEqual(index.commands, []);
});

test("citation keys come from bib files", () => {
  const index = indexProject({
    "refs.bib": [
      "@article{knuth1984,",
      "  author = {Knuth},",
      "}",
      "@book{lamport1986, title = {LaTeX} }",
    ].join("\n"),
  });

  assert.deepEqual(index.citations.map((c) => c.name), ["knuth1984", "lamport1986"]);
});

test("bib entries that declare no key are skipped", () => {
  // @string and @preamble are declarations, but not of anything citable.
  const index = indexProject({
    "refs.bib": '@string{jgr = "J. Geophys. Res."}\n@preamble{ "\\newcommand{\\x}{y}" }\n@misc{real,\n}\n',
  });

  assert.deepEqual(index.citations.map((c) => c.name), ["real"]);
});

test("files that declare nothing relevant are skipped", () => {
  const index = indexProject({
    "figure.png": "\\label{not-really}",
    "notes.md": "\\label{also-not}",
  });

  assert.deepEqual(index.labels, []);
});

test("a project that declares nothing yields empty lists, not undefined", () => {
  const index = indexProject({ "main.tex": "Hello.\n" });

  assert.deepEqual(index, { labels: [], citations: [], commands: [], environments: [] });
});
