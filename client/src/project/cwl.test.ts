import assert from "node:assert/strict";
import { test } from "node:test";

import { parseCwl } from "./cwl.ts";

test("a command's arguments keep the names the corpus gave them", () => {
  const pkg = parseCwl("amsmath", "\\frac{num}{den}\n\\sqrt[root]{arg}\n");

  assert.deepEqual(pkg.commands[0], {
    name: "frac",
    arguments: [
      { optional: false, name: "num", hint: null, keyvals: false },
      { optional: false, name: "den", hint: null, keyvals: false },
    ],
    classification: {
      unusual: false, hidden: false, mathOnly: false, textOnly: false,
      tabularOnly: false, role: null, environments: [],
    },
  });

  // The bracketed one is the optional argument, and order is preserved.
  assert.deepEqual(
    pkg.commands[1]?.arguments,
    [
      { optional: true, name: "root", hint: null, keyvals: false },
      { optional: false, name: "arg", hint: null, keyvals: false },
    ],
  );
});

test("classification letters are read", () => {
  const pkg = parseCwl("test", [
    "\\alpha#m",
    "\\toprule#t",
    "\\obscure#*",
    "\\internal#S",
    "\\ref{key}#r",
    "\\cite{key}#c",
    "\\label{key}#l",
  ].join("\n"));

  const by = new Map(pkg.commands.map((c) => [c.name, c.classification]));
  assert.equal(by.get("alpha")?.mathOnly, true);
  assert.equal(by.get("toprule")?.tabularOnly, true);
  assert.equal(by.get("obscure")?.unusual, true);
  assert.equal(by.get("internal")?.hidden, true);
  assert.equal(by.get("ref")?.role, "ref");
  assert.equal(by.get("cite")?.role, "cite");
  assert.equal(by.get("label")?.role, "label");
});

test("a hash inside an argument is not a classification", () => {
  // `\newcommand{cmd}{def}#d` classifies; the braces before it do not.
  const pkg = parseCwl("test", "\\newcommand{cmd}{def}#d\n");

  assert.equal(pkg.commands[0]?.name, "newcommand");
  assert.equal(pkg.commands[0]?.arguments.length, 2);
  assert.equal(pkg.commands[0]?.classification.role, "definition");
});

test("environments come from their begin line, and end lines are dropped", () => {
  const pkg = parseCwl("amsmath", [
    "\\begin{alignat}{ncols}#\\math,array",
    "\\end{alignat}",
    "\\begin{aligned}",
  ].join("\n"));

  assert.deepEqual(pkg.environments.map((e) => e.name), ["alignat", "aligned"]);
  // An environment's own arguments are what follows its \begin{…}.
  assert.deepEqual(pkg.environments[0]?.arguments, [
    { optional: false, name: "ncols", hint: null, keyvals: false },
  ]);
  // `#\math,array` names environments rather than classifying with letters.
  assert.deepEqual(pkg.environments[0]?.classification.environments, ["math", "array"]);
  assert.equal(pkg.environments.length, 2, "the \\end line must not add a third");
});

test("placeholder and keyval markers are not part of an argument's name", () => {
  const pkg = parseCwl("graphicx", "\\includegraphics[options%keyvals]{file}\n");

  assert.deepEqual(pkg.commands[0]?.arguments, [
    { optional: true, name: "options", hint: null, keyvals: true },
    { optional: false, name: "file", hint: null, keyvals: false },
  ]);
});

test("a type annotation is read as a hint, not as part of the name", () => {
  // `%l` says the argument is a length; it is not what the argument is called.
  const pkg = parseCwl("graphics", "\\resizebox{h-length%l}{v-length%l}{text}\n\\frac{num%:translatable}{den}\n");

  assert.deepEqual(pkg.commands[0]?.arguments.map((a) => [a.name, a.hint]), [
    ["h-length", "l"],
    ["v-length", "l"],
    ["text", null],
  ]);
  assert.deepEqual(pkg.commands[1]?.arguments.map((a) => [a.name, a.hint]), [
    ["num", "translatable"],
    ["den", null],
  ]);
});

test("keys carry their types and their enumerated values", () => {
  const pkg = parseCwl("graphicx", [
    "#keyvals:\\includegraphics",
    "width=##L",
    "keepaspectratio#true,false",
    "draft",
    "#endkeyvals",
  ].join("\n"));

  assert.deepEqual(pkg.keys["includegraphics"], [
    { name: "width", values: [], takesValue: true, hint: "length" },
    { name: "keepaspectratio", values: ["true", "false"], takesValue: true, hint: null },
    // A bare key is a flag, and offering it an `=` would be wrong.
    { name: "draft", values: [], takesValue: false, hint: null },
  ]);
});

test("a braced value keeps its braces", () => {
  // `alt={%<alt text%>}` means you write `alt={something}`: the braces are
  // literal. They are not part of what to fill in, and a snippet field cannot
  // contain one — `alt=${{alt text}}` ends the field at the first brace.
  const pkg = parseCwl("graphicx", [
    "#keyvals:\\includegraphics",
    "alt={%<alt text%>}",
    "#endkeyvals",
  ].join("\n"));

  assert.deepEqual(pkg.keys["includegraphics"], [
    { name: "alt", values: [], takesValue: true, hint: "{alt text}" },
  ]);
});

test("placeholder markers do not leak into keys", () => {
  const pkg = parseCwl("tikz", [
    "#keyvals:\\begin{tikzpicture}",
    // Two placeholders with a literal colon, not one placeholder oddly named.
    "domain=%<start%>:%<end%>",
    // A key name with a variable part in it.
    "level %<n%>/.style={%<options%>}",
    // Not a key at all: the block accepts any colour here.
    "%color",
    "#endkeyvals",
  ].join("\n"));

  const keys = pkg.keys["begin{tikzpicture}"] ?? [];
  assert.deepEqual(keys.map((k) => k.name), ["domain", "level n/.style"]);
  assert.equal(keys[0]?.hint, "start:end");
  assert.ok(!keys.some((k) => k.name.includes("%")), JSON.stringify(keys));
});

test("a block serving several commands files its keys under every one", () => {
  // TikZ declares one block for twenty-two owners at once. Keeping only the
  // last discarded 10,341 owner associations across the corpus, which is why
  // \begin{tikzpicture}[ had no options: they were filed under \arrowreversed.
  const pkg = parseCwl("tikz", [
    "#keyvals:\\begin{tikzpicture}#c,\\tikz#c,\\draw#c,\\node#c",
    "scale=%<factor%>",
    "thick",
    "#endkeyvals",
  ].join("\n"));

  assert.deepEqual(
    Object.keys(pkg.keys).sort(),
    ["begin{tikzpicture}", "draw", "node", "tikz"],
  );
  for (const owner of Object.keys(pkg.keys)) {
    assert.deepEqual(pkg.keys[owner]?.map((k) => k.name), ["scale", "thick"], owner);
  }
});

test("an enumeration written with an equals keeps its values", () => {
  // The `=` says the value is written with one. It does not mean there is no
  // list — five and a half thousand lines in the corpus are spelled this way,
  // and reading them as valueless threw all of it away.
  const pkg = parseCwl("hyperref", [
    "#keyvals:\\hypersetup,\\usepackage/hyperref#c",
    "backref=#section,slide,page,none,false",
    "citecolor=#%color",
    "baseurl=%<URL%>",
    "bookmarkstype=",
    "#endkeyvals",
  ].join("\n"));

  assert.deepEqual(pkg.keys["usepackage/hyperref"], [
    { name: "backref", values: ["section", "slide", "page", "none", "false"], takesValue: true, hint: null },
    { name: "citecolor", values: [], takesValue: true, hint: "color" },
    { name: "baseurl", values: [], takesValue: true, hint: "URL" },
    { name: "bookmarkstype", values: [], takesValue: true, hint: null },
  ]);
});

test("dependencies are collected", () => {
  const pkg = parseCwl("tikz", "#include:pgf\n#include:pgffor\n\\usetikzlibrary{list}\n");

  assert.deepEqual(pkg.includes, ["pgf", "pgffor"]);
});

test("comments and unknown directives are ignored rather than misread", () => {
  const pkg = parseCwl("test", [
    "# graphicx package",
    "# muzimuzhi/10 Aug 2019",
    "#repl:ae ä",
    "#ifOption:draft",
    "\\conditional{arg}",
    "#endif",
  ].join("\n"));

  assert.deepEqual(pkg.commands.map((c) => c.name), ["conditional"]);
  assert.deepEqual(pkg.includes, []);
});
