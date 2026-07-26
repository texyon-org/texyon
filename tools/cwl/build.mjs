/**
 * Turn TeXstudio's completion word lists into something the editor can load.
 *
 * The corpus is fetched by fetch.sh and is deliberately not vendored: its
 * licence is unresolved, so nothing derived from it is committed either. This
 * writes into client/public/cwl/, which is gitignored.
 *
 * Three kinds of output. One file per package, loaded when a document says
 * \usepackage{that}; one index from command name to the packages that provide
 * it, which is what lets the editor say "this needs \usepackage{siunitx}"
 * about a command that does not exist yet; and one list of every package there
 * is, which is what \usepackage{…} completes against.
 */
import { mkdirSync, readdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { parseCwl } from "../../client/src/project/cwl.ts";

const here = dirname(fileURLToPath(import.meta.url));
const source = join(here, "texstudio", "completion");
const out = join(here, "..", "..", "client", "public", "cwl");

// Empty it rather than remove it. A dev server watching public/ keeps a
// handle on this directory, and replacing it with a new one of the same name
// leaves that server serving its index.html fallback for every file in here
// until it is restarted — which looks exactly like a corpus that was never
// built.
mkdirSync(out, { recursive: true });
for (const entry of readdirSync(out)) {
  rmSync(join(out, entry), { recursive: true, force: true });
}

/** command name -> packages that provide it */
const provides = new Map();

/**
 * How many packages load each package.
 *
 * Several packages define the same command — `\SI` comes from siunitx, and
 * also from algobox, ascii and unitsdef — so "which package provides this"
 * needs an answer rather than the first one alphabetically. How many other
 * packages depend on a package is a decent proxy for how likely it is the one
 * meant: siunitx has 42 inbound includes, algobox has none.
 */
const dependedOn = new Map();
let commands = 0;
let environments = 0;
let keys = 0;

const files = readdirSync(source).filter((name) => name.endsWith(".cwl"));

// A first pass, because ranking a package needs to know about all the others.
for (const file of files) {
  for (const line of readFileSync(join(source, file), "utf8").split("\n")) {
    const include = line.trim().match(/^#include:(.+)$/);
    if (include?.[1] === undefined) continue;
    const name = include[1].trim();
    dependedOn.set(name, (dependedOn.get(name) ?? 0) + 1);
  }
}

for (const file of files) {
  const name = file.replace(/\.cwl$/, "");
  const pkg = parseCwl(name, readFileSync(join(source, file), "utf8"));

  commands += pkg.commands.length;
  environments += pkg.environments.length;
  keys += Object.values(pkg.keys).reduce((n, list) => n + list.length, 0);

  // A command nobody should be offered is not worth carrying to the browser.
  pkg.commands = pkg.commands.filter((c) => !c.classification.hidden);
  pkg.environments = pkg.environments.filter((e) => !e.classification.hidden);

  writeFileSync(join(out, `${name}.json`), JSON.stringify(pkg));

  for (const command of pkg.commands) {
    // Rarely-used commands are indexed too. They are not worth *offering*, but
    // "which package provides this" is a different question: siunitx marks \SI
    // rare because it deprecated it, and answering "algobox" instead — the only
    // other package that defines it — would be worse than answering nothing.
    let owners = provides.get(command.name);
    if (owners === undefined) provides.set(command.name, (owners = []));
    if (!owners.includes(name)) owners.push(name);
  }
}

// Most-depended-upon first, and only a few: a suggestion listing every package
// that happens to define a name is not a suggestion.
for (const owners of provides.values()) {
  owners.sort((left, right) => (dependedOn.get(right) ?? 0) - (dependedOn.get(left) ?? 0));
  owners.splice(3);
}

writeFileSync(join(out, "index.json"), JSON.stringify(Object.fromEntries(provides)));

// Every package there is, most-depended-upon first, so that completing
// `\usepackage{` offers amsmath before amsmath-ish obscurities. Classes are
// listed separately: `\documentclass{` and `\usepackage{` do not take the
// same names, and the corpus keeps classes under a `class-` prefix.
const listed = (names) =>
  names
    .slice()
    .sort((left, right) => {
      const weight = (dependedOn.get(right) ?? 0) - (dependedOn.get(left) ?? 0);
      return weight !== 0 ? weight : left.localeCompare(right);
    });

const all = files.map((file) => file.replace(/\.cwl$/, ""));
writeFileSync(
  join(out, "packages.json"),
  JSON.stringify({
    packages: listed(all.filter((name) => !name.startsWith("class-"))),
    classes: listed(all.filter((name) => name.startsWith("class-"))).map((name) =>
      name.slice("class-".length),
    ),
  }),
);

const size = (path) => readdirSync(out).reduce((n, f) => n + readFileSync(join(out, f)).length, 0);
console.log(`packages     ${files.length}`);
console.log(`commands     ${commands}`);
console.log(`environments ${environments}`);
console.log(`keys         ${keys}`);
console.log(`index        ${provides.size} names, ${(readFileSync(join(out, "index.json")).length / 1e6).toFixed(1)} MB`);
console.log(`packages.json ${(readFileSync(join(out, "packages.json")).length / 1024).toFixed(0)} KB`);
console.log(`total        ${(size() / 1e6).toFixed(1)} MB in ${readdirSync(out).length} files`);
