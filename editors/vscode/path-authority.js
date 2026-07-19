"use strict";

const fs = require("fs");
const path = require("path");

const CHILD_EXTENSIONS = new Set([".tex", ".ltx", ".sty", ".cls", ".def"]);

/** Resolve a path through its nearest existing ancestor. */
function resolveAuthority(candidate, fileSystem = fs) {
  let existing = path.resolve(candidate);
  const suffix = [];
  while (!fileSystem.existsSync(existing)) {
    const parent = path.dirname(existing);
    if (parent === existing) throw new Error(`no existing ancestor for ${candidate}`);
    suffix.unshift(path.basename(existing));
    existing = parent;
  }
  const realpath = fileSystem.realpathSync.native || fileSystem.realpathSync;
  return path.resolve(realpath.call(fileSystem, existing), ...suffix);
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

/** Return a normalized server child name, or null when authority is ineligible. */
function childLogicalPath(primaryPath, candidatePath, fileSystem = fs) {
  if (!CHILD_EXTENSIONS.has(path.extname(candidatePath).toLowerCase())) return null;
  const root = resolveAuthority(path.dirname(primaryPath), fileSystem);
  const primary = resolveAuthority(primaryPath, fileSystem);
  const candidate = resolveAuthority(candidatePath, fileSystem);
  if (candidate === primary || !isWithin(root, candidate)) return null;
  return path.relative(root, candidate).split(path.sep).join("/");
}

/** Resolve a server-reported source name without escaping the document root. */
function sourceFilePath(primaryPath, sourceName, fileSystem = fs) {
  if (typeof sourceName !== "string" || sourceName.length === 0) {
    throw new Error("source name is empty");
  }
  const root = resolveAuthority(path.dirname(primaryPath), fileSystem);
  let candidate = sourceName.replaceAll("/", path.sep);
  if (!path.extname(candidate)) candidate += ".tex";
  candidate = resolveAuthority(path.isAbsolute(candidate) ? candidate : path.join(root, candidate), fileSystem);
  if (!isWithin(root, candidate)) throw new Error(`source escapes document root: ${sourceName}`);
  return candidate;
}

module.exports = { childLogicalPath, isWithin, resolveAuthority, sourceFilePath };
