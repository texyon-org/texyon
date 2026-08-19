/**
 * Keep a project's server copy caught up with its workspace.
 *
 * The workspace is the authority on the text; this watches it and writes what
 * changed, a moment after it stops changing. It is deliberately not the other
 * way around — nothing here ever writes *into* the workspace, so a save that
 * races a keystroke can lose the race safely: the path just stays dirty and is
 * written again.
 *
 * A rename needs no special handling. The workspace reports it as one change
 * that removed `from` and dirtied `to`, and replaying exactly that against the
 * server — delete one path, write the other — is what a rename is there too.
 */

import type { Workspace } from "../workspace/Workspace";

/** The two writes a sync can make, already bound to a project. */
export interface SyncApi {
  writeFile(path: string, text: string): Promise<unknown>;
  deleteFile(path: string): Promise<void>;
}

/**
 * Where the server copy stands relative to the workspace.
 *
 * `signed-out` is separate from `failed` because they call for different
 * behavior: a failure is retried on a timer, while a lapsed session is not —
 * retrying an expired cookie every few seconds would never succeed, so it
 * waits for the person to sign back in and `retry()` to be called.
 */
export type SyncStatus = "synced" | "dirty" | "saving" | "failed" | "signed-out";

export class ProjectSync {
  #workspace: Workspace;
  #api: SyncApi;
  #onStatus: (status: SyncStatus) => void;
  #debounceMs: number;
  #retryMs: number;

  /** Paths removed locally and not yet removed from the server. */
  #deletes = new Set<string>();
  #timer: ReturnType<typeof setTimeout> | null = null;
  #inFlight = false;
  /** The workspace moved while a flush was writing; go around again. */
  #again = false;
  #status: SyncStatus = "synced";
  #stopped = false;
  #unsubscribe: () => void;

  constructor(
    workspace: Workspace,
    api: SyncApi,
    onStatus: (status: SyncStatus) => void,
    { debounceMs = 1000, retryMs = 5000 } = {},
  ) {
    this.#workspace = workspace;
    this.#api = api;
    this.#onStatus = onStatus;
    this.#debounceMs = debounceMs;
    this.#retryMs = retryMs;

    this.#unsubscribe = workspace.subscribe((change) => {
      for (const path of change.removed) this.#deletes.add(path);
      if (this.#inFlight) {
        this.#again = true;
      } else {
        this.#emit("dirty");
        this.#schedule(this.#debounceMs);
      }
    });

    // The workspace may already be ahead of the server — a project seeded with
    // its first file arrives here dirty.
    if (this.pending) {
      this.#emit("dirty");
      this.#schedule(this.#debounceMs);
    }
  }

  /** Whether anything local is not yet on the server. */
  get pending(): boolean {
    return this.#workspace.dirty().length > 0 || this.#deletes.size > 0;
  }

  /** Stop watching and let go of any timers. In-flight requests finish alone. */
  stop(): void {
    this.#stopped = true;
    this.#unsubscribe();
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = null;
  }

  /** Try again now — after signing back in, or without waiting out the timer. */
  retry(): void {
    void this.flush();
  }

  /**
   * Write everything pending, now.
   *
   * Deletes go first: a path that was removed and then recreated must end up
   * existing, and delete-then-write is the order with that outcome.
   */
  async flush(): Promise<void> {
    if (this.#inFlight || this.#stopped) {
      this.#again = this.#inFlight;
      return;
    }
    if (!this.pending) {
      this.#emit("synced");
      return;
    }

    this.#inFlight = true;
    this.#emit("saving");
    try {
      do {
        this.#again = false;

        for (const path of [...this.#deletes]) {
          try {
            await this.#api.deleteFile(path);
          } catch (error) {
            // Already gone is the outcome a delete wanted.
            if (httpStatus(error) !== 404) throw error;
          }
          this.#deletes.delete(path);
        }

        for (const path of this.#workspace.dirty()) {
          const text = this.#workspace.read(path);
          if (text === null) continue;
          await this.#api.writeFile(path, text);
          // Only if the workspace still says what was sent: a keystroke during
          // the request means the path is dirty again, not saved.
          if (this.#workspace.read(path) === text) this.#workspace.markSaved([path]);
        }
      } while (this.#again && !this.#stopped);

      this.#emit(this.pending ? "dirty" : "synced");
      if (this.pending) this.#schedule(this.#debounceMs);
    } catch (error) {
      const status = httpStatus(error);
      if (status === 401 || status === 419) {
        this.#emit("signed-out");
      } else {
        this.#emit("failed");
        this.#schedule(this.#retryMs);
      }
    } finally {
      this.#inFlight = false;
    }
  }

  #schedule(afterMs: number): void {
    if (this.#stopped) return;
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      this.#timer = null;
      void this.flush();
    }, afterMs);
  }

  #emit(status: SyncStatus): void {
    if (status === this.#status || this.#stopped) return;
    this.#status = status;
    this.#onStatus(status);
  }
}

/**
 * The HTTP status a failed request carries, if it carries one.
 *
 * Structural rather than `instanceof ApiError`, so this module depends only on
 * its `SyncApi` — which is also what lets the tests hand it plain fakes.
 */
function httpStatus(error: unknown): number | null {
  if (typeof error === "object" && error !== null && "status" in error) {
    const status = (error as { status: unknown }).status;
    if (typeof status === "number") return status;
  }
  return null;
}
