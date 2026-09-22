import * as Y from "yjs";

import type { Workspace } from "../workspace/Workspace";
import { binariesOf, filesOf, resolvePaths } from "./model.ts";
import type { BinaryFile } from "./model.ts";

/** Paths that moved or went away in one mirror pass, for whoever has one of them open. */
export interface Moves {
  renamed: ReadonlyMap<string, string>;
  removed: readonly string[];
}

/**
 * Keeps a Workspace a faithful copy of a project's shared document.
 *
 * The document is the authority. Everything that edits files goes through the
 * methods here, which change the document, and the workspace follows from the
 * document's own events, so local and remote edits take the same path to the
 * engine.
 */
export class CollabBinding {
  readonly doc: Y.Doc;
  #workspace: Workspace;
  #fetchBinary: (digest: string) => Promise<Uint8Array>;
  /** id to the path it currently resolves to. */
  #paths = new Map<string, string>();
  #ids = new Map<string, string>();
  /** The digest each binary path in the workspace holds. */
  #mirrored = new Map<string, string>();
  #bytes = new Map<string, Uint8Array>();
  #listeners = new Set<(moves: Moves) => void>();
  #started = false;
  #onFiles: (events: Y.YEvent<Y.AbstractType<unknown>>[]) => void;
  #onBinaries: () => void;

  constructor(doc: Y.Doc, workspace: Workspace, fetchBinary: (digest: string) => Promise<Uint8Array>) {
    this.doc = doc;
    this.#workspace = workspace;
    this.#fetchBinary = fetchBinary;

    this.#onFiles = (events) => {
      if (!this.#started) return;
      const structural = events.some(
        (event) => event.target === filesOf(doc) || (event.target instanceof Y.Map && event.keysChanged.has("path")),
      );
      if (structural) {
        this.#reconcile();
        return;
      }
      const edited = new Set<string>();
      for (const event of events) {
        if (event.target instanceof Y.Text && typeof event.path[0] === "string") edited.add(event.path[0]);
      }
      for (const id of edited) this.#mirrorText(id);
    };
    this.#onBinaries = () => {
      if (this.#started) this.#reconcile();
    };
    filesOf(doc).observeDeep(this.#onFiles);
    binariesOf(doc).observe(this.#onBinaries);
  }

  /** Begin mirroring, once the document has synced with the server. */
  start(): void {
    this.#started = true;
    this.#reconcile();
  }

  destroy(): void {
    filesOf(this.doc).unobserveDeep(this.#onFiles);
    binariesOf(this.doc).unobserve(this.#onBinaries);
    this.#listeners.clear();
  }

  /** Told when a mirror pass renamed or removed paths. */
  onMoves(listener: (moves: Moves) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  has(path: string): boolean {
    return this.#ids.has(path);
  }

  /** The shared text behind a path, for the editor to bind to. */
  textAt(path: string): Y.Text | null {
    const id = this.#ids.get(path);
    const text = id === undefined ? undefined : filesOf(this.doc).get(id)?.get("text");
    return text instanceof Y.Text ? text : null;
  }

  create(path: string, text: string): boolean {
    if (this.#ids.has(path)) return false;
    this.#insert(path, text);
    return true;
  }

  /** Replace a text file's contents, creating it if needed. A binary on that path is replaced. */
  writeText(path: string, text: string): void {
    const existing = this.textAt(path);
    if (existing === null) {
      this.doc.transact(() => {
        this.#removeBinary(path);
        this.#insert(path, text);
      });
      return;
    }
    if (existing.toString() === text) return;
    this.doc.transact(() => {
      existing.delete(0, existing.length);
      existing.insert(0, text);
    });
  }

  /** Point a path at uploaded bytes. A text file on that path is replaced. */
  setBinary(path: string, binary: Omit<BinaryFile, "path">, bytes: Uint8Array): void {
    this.#bytes.set(binary.digest, bytes);
    this.doc.transact(() => {
      const id = this.#ids.get(path);
      if (id !== undefined && filesOf(this.doc).has(id)) filesOf(this.doc).delete(id);
      const binaryId = id !== undefined && binariesOf(this.doc).has(id) ? id : crypto.randomUUID();
      binariesOf(this.doc).set(binaryId, { path, digest: binary.digest, size: binary.size });
    });
  }

  remove(path: string): boolean {
    const id = this.#ids.get(path);
    if (id === undefined) return false;
    this.doc.transact(() => {
      filesOf(this.doc).delete(id);
      binariesOf(this.doc).delete(id);
    });
    return true;
  }

  rename(from: string, to: string): boolean {
    const id = this.#ids.get(from);
    if (id === undefined || from === to || this.#ids.has(to)) return false;
    const entry = filesOf(this.doc).get(id);
    if (entry !== undefined) {
      entry.set("path", to);
      return true;
    }
    const binary = binariesOf(this.doc).get(id);
    if (binary === undefined) return false;
    binariesOf(this.doc).set(id, { ...binary, path: to });
    return true;
  }

  #insert(path: string, text: string): void {
    const entry = new Y.Map<unknown>();
    entry.set("path", path);
    entry.set("text", new Y.Text(text));
    filesOf(this.doc).set(crypto.randomUUID(), entry);
  }

  #removeBinary(path: string): void {
    const id = this.#ids.get(path);
    if (id !== undefined) binariesOf(this.doc).delete(id);
  }

  #mirrorText(id: string): void {
    const path = this.#paths.get(id);
    const text = filesOf(this.doc).get(id)?.get("text");
    if (path === undefined || !(text instanceof Y.Text)) return;
    if (this.#workspace.write(path, text.toString())) this.#workspace.markSaved([path]);
  }

  /** Bring the workspace's whole file list in line with the document. */
  #reconcile(): void {
    const texts: { id: string; path: string }[] = [];
    filesOf(this.doc).forEach((entry, id) => {
      const path = entry.get("path");
      if (typeof path === "string" && entry.get("text") instanceof Y.Text) texts.push({ id, path });
    });
    const binaries: { id: string; path: string; binary: BinaryFile }[] = [];
    binariesOf(this.doc).forEach((binary, id) => binaries.push({ id, path: binary.path, binary }));

    const resolved = resolvePaths([...texts, ...binaries]);
    const before = this.#paths;
    this.#paths = resolved;
    this.#ids = new Map([...resolved].map(([id, path]) => [path, id]));

    const renamed = new Map<string, string>();
    for (const [id, path] of before) {
      const now = resolved.get(id);
      if (now !== undefined && now !== path) renamed.set(path, now);
    }

    const current = new Set(resolved.values());
    const removed: string[] = [];
    for (const path of this.#workspace.paths()) {
      if (!current.has(path)) {
        this.#workspace.remove(path);
        this.#mirrored.delete(path);
        if (!renamed.has(path)) removed.push(path);
      }
    }

    for (const { id } of texts) this.#mirrorText(id);
    for (const { id, binary } of binaries) {
      const path = resolved.get(id);
      if (path !== undefined) void this.#mirrorBinary(path, binary.digest);
    }

    if (renamed.size > 0 || removed.length > 0) {
      for (const listener of [...this.#listeners]) listener({ renamed, removed });
    }
  }

  async #mirrorBinary(path: string, digest: string): Promise<void> {
    if (this.#mirrored.get(path) === digest) return;
    this.#mirrored.set(path, digest);
    let bytes = this.#bytes.get(digest);
    if (bytes === undefined) {
      try {
        bytes = await this.#fetchBinary(digest);
      } catch {
        this.#mirrored.delete(path);
        return;
      }
      this.#bytes.set(digest, bytes);
    }
    // Something newer may have landed on this path while the bytes were in flight.
    if (this.#mirrored.get(path) !== digest) return;
    if (this.#workspace.write(path, bytes)) this.#workspace.markSaved([path]);
  }
}
