import assert from "node:assert/strict";
import { test } from "node:test";

import type { CwlPackage } from "./cwl.ts";
import { availableCommand, mergeCommands, mergeEnvironments, packagesUsed, resolvePackages } from "./commands.ts";
import type { EngineCommand } from "./commands.ts";

const engine = (name: string, arity = 0, optional = false): EngineCommand => ({
  name,
  arity,
  optional,
  kind: "macro",
});

const pkg = (name: string, commands: CwlPackage["commands"], includes: string[] = []): CwlPackage => ({
  name,
  includes,
  commands,
  environments: [],
  keys: {},
});

const entry = (name: string, args: [string, boolean][] = []) => ({
  name,
  arguments: args.map(([argName, optional]) => ({
    optional,
    name: argName,
    hint: null,
    keyvals: false,
  })),
  classification: {
    unusual: false, hidden: false, mathOnly: false, textOnly: false,
    tabularOnly: false, role: null as null, environments: [],
  },
});

test("a command in both sources keeps the engine's arity and the corpus's names", () => {
  const merged = mergeCommands(
    [engine("frac", 2)],
    [pkg("latex-document", [entry("frac", [["num", false], ["den", false]])])],
  );

  assert.equal(merged[0]?.tier, "described");
  assert.equal(merged[0]?.arity, 2);
  assert.deepEqual(merged[0]?.signatures?.[0]?.map((a) => a.name), ["num", "den"]);
});

test("the corpus supplies the count the engine could not recover", () => {
  // \section is built from \@startsection, which reads its arguments off the
  // input stream — there is no parameter text, so the engine reports zero and
  // the corpus is the only source of an answer.
  const merged = mergeCommands(
    [engine("section", 0)],
    [pkg("latex-document", [entry("section", [["short title", true], ["title", false]])])],
  );

  assert.equal(merged[0]?.arity, 2);
  assert.equal(merged[0]?.optional, true, "the corpus says the first argument is optional");
});

test("the engine wins on arity when it has one", () => {
  // A document that redefines a command has changed its arity, and the corpus
  // describes the original. What is actually loaded is what matters.
  const merged = mergeCommands(
    [engine("frac", 3)],
    [pkg("latex-document", [entry("frac", [["num", false], ["den", false]])])],
  );

  assert.equal(merged[0]?.arity, 3);
});

test("every way of writing a command is kept, not just the first", () => {
  // \section is listed four times in the corpus — with and without a star,
  // with and without a short title. Keeping only the first would silently drop
  // the optional argument, which is the one people forget.
  const merged = mergeCommands(
    [engine("section", 0)],
    [
      pkg("latex-document", [
        entry("section", [["title", false]]),
        entry("section", [["short title", true], ["title", false]]),
      ]),
    ],
  );

  assert.equal(merged[0]?.signatures?.length, 2);
  // The fullest describes it, since it is the only one showing every argument.
  assert.equal(merged[0]?.arity, 2);
});

test("the same signature from two packages is not counted twice", () => {
  const merged = mergeCommands(
    [engine("frac", 2)],
    [
      pkg("latex-document", [entry("frac", [["num", false], ["den", false]])]),
      pkg("optex", [entry("frac", [["num", false], ["den", false]])]),
    ],
  );

  assert.equal(merged[0]?.signatures?.length, 1);
});

test("a command the corpus never heard of is still offered", () => {
  // Someone's own \newcommand, or a package among the thousands the corpus
  // does not cover.
  const merged = mergeCommands([engine("vect", 1)], []);

  assert.equal(merged[0]?.tier, "bare");
  assert.equal(merged[0]?.arity, 1);
  assert.equal(merged[0]?.signatures, null);
});

test("a command the document does not have is offered as a package to load", () => {
  const providers = new Map([["SI", "siunitx"], ["frac", "latex-document"]]);
  const defined = new Set(["frac"]);

  const suggestion = availableCommand("SI", providers, defined);
  assert.equal(suggestion?.tier, "available");
  assert.equal(suggestion?.from, "siunitx");
});

test("a command that already exists is not something to go and load", () => {
  const providers = new Map([["frac", "latex-document"]]);

  assert.equal(availableCommand("frac", providers, new Set(["frac"])), null);
});

test("a command no package here provides has nothing to suggest", () => {
  assert.equal(availableCommand("nope", new Map(), new Set()), null);
});

test("the merged set holds only what the document has", () => {
  // The sixty thousand commands nobody loaded are a lookup, not a list: they
  // must not turn up in the merge, which is rebuilt on every compile.
  const merged = mergeCommands([engine("frac", 2)], []);

  assert.deepEqual(merged.map((c) => c.name), ["frac"]);
});

test("packages are read from the source, class included", () => {
  const used = packagesUsed(
    "\\documentclass[11pt]{article}\n\\usepackage{amsmath}\n\\usepackage[utf8]{inputenc, fontenc}\n",
  );

  assert.ok(used.includes("class-article"), "the class is filed under a prefix");
  assert.ok(used.includes("amsmath"));
  // One \usepackage may load several.
  assert.ok(used.includes("inputenc"));
  assert.ok(used.includes("fontenc"));
  // Always present, whether or not anything says so.
  assert.ok(used.includes("latex-document"));
});

test("transitive loads reach everything, once each", async () => {
  const corpus: Record<string, CwlPackage> = {
    tikz: pkg("tikz", [], ["pgf"]),
    // pgf loads tikz back: a cycle, which must terminate rather than recur.
    pgf: pkg("pgf", [], ["keyval", "tikz"]),
    keyval: pkg("keyval", []),
  };

  const resolved = await resolvePackages(["tikz"], async (name) => corpus[name] ?? null);

  assert.deepEqual(resolved.map((p) => p.name), ["tikz", "pgf", "keyval"]);
});

test("a package the corpus does not describe is skipped, not fatal", async () => {
  const resolved = await resolvePackages(["nonexistent", "amsmath"], async (name) =>
    name === "amsmath" ? pkg("amsmath", [entry("binom")]) : null,
  );

  assert.deepEqual(resolved.map((p) => p.name), ["amsmath"]);
});

const BARE = {
  unusual: false,
  hidden: false,
  mathOnly: false,
  textOnly: false,
  tabularOnly: false,
  role: null,
  environments: [],
} as const;

test("environments come from the packages the document loads", () => {
  // Nothing offered these before: \begin{ knew only what the project declared
  // and a short hardcoded list, so no package environment was ever reachable.
  const merged = mergeEnvironments([
    {
      name: "tikz",
      includes: [],
      commands: [],
      environments: [
        { name: "tikzpicture", arguments: [], classification: BARE },
        {
          name: "tikzpicture",
          arguments: [{ optional: true, name: "options", hint: null, keyvals: true }],
          classification: BARE,
        },
      ],
      keys: {},
    },
  ]);

  assert.deepEqual(merged.map((e) => e.name), ["tikzpicture"]);
  // The richest description wins, so accepting it leaves you in the options.
  assert.deepEqual(merged[0]?.arguments.map((a) => a.name), ["options"]);
  assert.equal(merged[0]?.from, "tikz");
});
