/**
 * The authority on what every file in a project currently says.
 *
 * One place owns the text, and everything else — the editor, the compiler, the
 * thing that saves — reads from it and is told when it changes. That is not
 * ceremony: it is the seam collaborative editing would replace. Today a
 * `Workspace` is a map of strings with one writer. Tomorrow its internals could
 * be a CRDT with several, and nothing outside this file would need to know,
 * because what leaves it is already the shape the compiler wants — every file's
 * complete current text, at a revision.
 *
 * The revision is what makes that work. It advances on every accepted change,
 * so a caller can ask "has anything moved since the version I compiled?"
 * without comparing the text, and a compile that finishes late can tell that it
 * is stale.
 */

/** Every file's complete current text, by path relative to the project root. */
export type ProjectFiles = Record<string, string>;

/** What changed, for a listener that wants to know more than "something". */
export interface WorkspaceChange {
  /** The revision the workspace is now at. */
  revision: number;
  /** Paths whose text differs from before this change. */
  changed: readonly string[];
  /** Paths the project no longer has. */
  removed: readonly string[];
}

/** Told when the workspace moves. */
export type WorkspaceListener = (change: WorkspaceChange) => void;

/** The files of one project, and who is allowed to say what they contain. */
export class Workspace {
  #files = new Map<string, string>();
  #revision = 0;
  #listeners = new Set<WorkspaceListener>();
  #dirty = new Set<string>();

  constructor(files: ProjectFiles = {}) {
    for (const [path, text] of Object.entries(files)) {
      this.#files.set(path, text);
    }
  }

  /** How many times the contents have been accepted as changed. */
  get revision(): number {
    return this.#revision;
  }

  /** Every path the project has, in a stable order. */
  paths(): string[] {
    return [...this.#files.keys()].sort();
  }

  has(path: string): boolean {
    return this.#files.has(path);
  }

  /** One file's text, or null if the project does not have it. */
  read(path: string): string | null {
    return this.#files.get(path) ?? null;
  }

  /**
   * Every file's complete current text.
   *
   * This is exactly what the engine's `compile` takes, which is not a
   * coincidence — handing over the whole project is the intended usage there,
   * and the engine diffs it against what it last saw.
   */
  snapshot(): ProjectFiles {
    return Object.fromEntries(this.#files);
  }

  /**
   * Paths written since they were last marked saved.
   *
   * The workspace tracks this rather than the thing that saves, because the
   * workspace is what knows when a write actually changed anything.
   */
  dirty(): string[] {
    return [...this.#dirty].sort();
  }

  /**
   * Replace one file's complete text.
   *
   * Writing text a file already has is not a change: it does not advance the
   * revision, does not mark anything dirty, and tells nobody. An editor that
   * echoes its own value back — which every editor does — would otherwise
   * recompile the document on every keystroke that changed nothing.
   */
  write(path: string, text: string): boolean {
    if (this.#files.get(path) === text) return false;

    this.#files.set(path, text);
    this.#dirty.add(path);
    this.#advance({ changed: [path], removed: [] });
    return true;
  }

  /** Add a file the project did not have. */
  create(path: string, text = ""): boolean {
    if (this.#files.has(path)) return false;
    return this.write(path, text);
  }

  remove(path: string): boolean {
    if (!this.#files.delete(path)) return false;

    this.#dirty.delete(path);
    this.#advance({ changed: [], removed: [path] });
    return true;
  }

  /**
   * Move a file, keeping its contents.
   *
   * One change rather than a remove and a create, so a listener never sees a
   * moment where the project has neither.
   */
  rename(from: string, to: string): boolean {
    const text = this.#files.get(from);
    if (text === undefined || from === to || this.#files.has(to)) return false;

    this.#files.delete(from);
    this.#files.set(to, text);
    this.#dirty.delete(from);
    this.#dirty.add(to);
    this.#advance({ changed: [to], removed: [from] });
    return true;
  }

  /**
   * Note that these paths now match what is stored.
   *
   * Takes the paths rather than clearing everything, because a save is of a
   * particular version and the user may have typed again while it was in
   * flight.
   */
  markSaved(paths: Iterable<string>): void {
    for (const path of paths) this.#dirty.delete(path);
  }

  /** Be told when the workspace moves. Returns a function that stops that. */
  subscribe(listener: WorkspaceListener): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #advance(change: Omit<WorkspaceChange, "revision">): void {
    this.#revision++;
    const full: WorkspaceChange = { ...change, revision: this.#revision };
    // A copy, so a listener that unsubscribes during dispatch does not change
    // the set being iterated.
    for (const listener of [...this.#listeners]) listener(full);
  }
}
