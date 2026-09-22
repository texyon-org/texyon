// A copy of collab/src/model.ts. The two must agree on the shape of a project's document.
import * as Y from "yjs";

export interface TextFile {
  path: string;
  text: string;
}

export interface BinaryFile {
  path: string;
  digest: string;
  size: number;
}

export interface Contents {
  files: TextFile[];
  binaries: BinaryFile[];
}

/** Files are keyed by a stable id, so a rename is a field change and concurrent edits survive it. */
export function filesOf(doc: Y.Doc): Y.Map<Y.Map<unknown>> {
  return doc.getMap("files");
}

export function binariesOf(doc: Y.Doc): Y.Map<BinaryFile> {
  return doc.getMap("binaries");
}

export function metaOf(doc: Y.Doc): Y.Map<string> {
  return doc.getMap("meta");
}

/** Fill an empty document from what the server stored, stamping a fresh epoch. */
export function seed(doc: Y.Doc, contents: Contents): string {
  const epoch = crypto.randomUUID();
  doc.transact(() => {
    for (const file of contents.files) {
      const entry = new Y.Map<unknown>();
      entry.set("path", file.path);
      entry.set("text", new Y.Text(file.text));
      filesOf(doc).set(crypto.randomUUID(), entry);
    }
    for (const binary of contents.binaries) {
      binariesOf(doc).set(crypto.randomUUID(), { path: binary.path, digest: binary.digest, size: binary.size });
    }
    metaOf(doc).set("epoch", epoch);
  });
  return epoch;
}

/** Two entries claiming one path keep it in id order, later ones get " (2)" and so on. */
export function resolvePaths(ids: readonly { id: string; path: string }[]): Map<string, string> {
  const resolved = new Map<string, string>();
  const taken = new Set<string>();
  for (const { id, path } of [...ids].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) {
    let candidate = path;
    for (let n = 2; taken.has(candidate); n++) candidate = withSuffix(path, n);
    taken.add(candidate);
    resolved.set(id, candidate);
  }
  return resolved;
}

function withSuffix(path: string, n: number): string {
  const slash = path.lastIndexOf("/");
  const dot = path.lastIndexOf(".");
  return dot > slash + 1 ? `${path.slice(0, dot)} (${n})${path.slice(dot)}` : `${path} (${n})`;
}

/** Everything in the document as plain files, paths already de-duplicated. */
export function contentsOf(doc: Y.Doc): Contents {
  const texts: { id: string; path: string; text: string }[] = [];
  filesOf(doc).forEach((entry, id) => {
    const path = entry.get("path");
    const text = entry.get("text");
    if (typeof path === "string" && text instanceof Y.Text) texts.push({ id, path, text: text.toString() });
  });
  const binaries: { id: string; binary: BinaryFile }[] = [];
  binariesOf(doc).forEach((binary, id) => binaries.push({ id, binary }));

  const paths = resolvePaths([
    ...texts.map(({ id, path }) => ({ id, path })),
    ...binaries.map(({ id, binary }) => ({ id, path: binary.path })),
  ]);
  return {
    files: texts.map(({ id, text }) => ({ path: paths.get(id) ?? "", text })),
    binaries: binaries.map(({ id, binary }) => ({ ...binary, path: paths.get(id) ?? "" })),
  };
}
