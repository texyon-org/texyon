import assert from "node:assert/strict";
import { test } from "node:test";

import type { MergedCommand } from "../project/commands.ts";
import { commandSnippet, describeSignature, environmentSnippet } from "./snippets.ts";

const arg = (name: string, optional = false) => ({ optional, name, hint: null, keyvals: false });

const command = (over: Partial<MergedCommand>): MergedCommand => ({
  name: "x",
  tier: "bare",
  signatures: null,
  arity: 0,
  optional: false,
  classification: null,
  from: null,
  ...over,
});

test("a described command becomes fields named after its arguments", () => {
  // Accepting \frac should leave you typing the numerator, not sitting after
  // the word "frac".
  const template = commandSnippet(
    command({ name: "frac", tier: "described", arity: 2, signatures: [[arg("num"), arg("den")]] }),
  );

  assert.equal(template, "frac\\{${num}\\}\\{${den}\\}");
});

test("an optional argument becomes a bracketed field", () => {
  const template = commandSnippet(
    command({
      name: "section",
      tier: "described",
      arity: 2,
      signatures: [[arg("short title", true), arg("title")]],
    }),
  );

  assert.equal(template, "section[${short title}]\\{${title}\\}");
});

test("a command with no corpus entry still gets a field per argument", () => {
  // Someone's own \newcommand{\vect}[1]: no names to offer, but the cursor
  // should still land inside the braces.
  assert.equal(commandSnippet(command({ name: "vect", arity: 1 })), "vect\\{${}\\}");
  assert.equal(commandSnippet(command({ name: "pair", arity: 2 })), "pair\\{${}\\}\\{${}\\}");
});

test("an engine-reported optional argument is bracketed too", () => {
  assert.equal(commandSnippet(command({ name: "foo", arity: 2, optional: true })), "foo[${}]\\{${}\\}");
});

test("a command that takes nothing gets no snippet at all", () => {
  // A snippet with no fields would leave a session to escape from for no gain.
  assert.equal(commandSnippet(command({ name: "alpha", arity: 0 })), null);
});

test("an environment writes its own end, with both names as one field", () => {
  // The same field twice is what makes renaming one rename the other; the
  // final cursor sits on the body line between them.
  const template = environmentSnippet("itemize");

  assert.equal(template, "${itemize}\\}\n\t${0}\n\\end\\{${itemize}\\}");
});

test("an environment's own arguments come before its body", () => {
  const template = environmentSnippet("alignat", [arg("ncols")]);

  assert.equal(template, "${alignat}\\}\\{${ncols}\\}\n\t${0}\n\\end\\{${alignat}\\}");
});

test("an environment's optional arguments are not written for it", () => {
  // `\begin{tikzpicture}[options]` is not a document that compiles: tikz reads
  // `options` as a key. A mandatory argument has to be filled in either way,
  // so it still gets a field.
  const template = environmentSnippet("tikzpicture", [arg("options", true)]);
  assert.equal(template, "${tikzpicture}\\}\n\t${0}\n\\end\\{${tikzpicture}\\}");

  const mixed = environmentSnippet("minipage", [arg("pos", true), arg("width")]);
  assert.ok(mixed.includes("${width}"), mixed);
  assert.ok(!mixed.includes("${pos}"), mixed);
});

test("a signature reads as it would be written", () => {
  assert.equal(
    describeSignature(command({ signatures: [[arg("num"), arg("den")]] })),
    "{num}{den}",
  );
  assert.equal(
    describeSignature(command({ signatures: [[arg("opt", true), arg("text")]] })),
    "[opt]{text}",
  );
  // Without names, the shape is still worth showing.
  assert.equal(describeSignature(command({ arity: 2 })), "{}{}");
  assert.equal(describeSignature(command({ arity: 2, optional: true })), "[]{}");
  assert.equal(describeSignature(command({ arity: 0 })), "");
});
