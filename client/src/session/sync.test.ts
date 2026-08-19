import assert from "node:assert/strict";
import { test } from "node:test";

import { Workspace } from "../workspace/Workspace.ts";
import { ProjectSync } from "./sync.ts";
import type { SyncStatus } from "./sync.ts";

/** A server that remembers what was done to it, and can be told to refuse. */
class FakeApi {
  files = new Map<string, string>();
  calls: string[] = [];
  /** The next requests fail with this status, until cleared. */
  failWith: number | null = null;

  async writeFile(path: string, text: string): Promise<void> {
    this.calls.push(`put ${path}`);
    this.#maybeFail();
    this.files.set(path, text);
  }

  async deleteFile(path: string): Promise<void> {
    this.calls.push(`delete ${path}`);
    this.#maybeFail();
    if (!this.files.delete(path)) {
      throw Object.assign(new Error("no such file"), { status: 404 });
    }
  }

  #maybeFail(): void {
    if (this.failWith !== null) {
      throw Object.assign(new Error(`HTTP ${this.failWith}`), { status: this.failWith });
    }
  }
}

/** A sync whose timers never matter, because the tests call flush() directly. */
function harness(files: Record<string, string> = {}) {
  const workspace = new Workspace(files);
  const api = new FakeApi();
  for (const [path, text] of Object.entries(files)) api.files.set(path, text);
  const statuses: SyncStatus[] = [];
  const sync = new ProjectSync(workspace, api, (status) => statuses.push(status), {
    debounceMs: 3_600_000,
    retryMs: 3_600_000,
  });
  return { workspace, api, statuses, sync };
}

test("writes dirty files and marks them saved", async () => {
  const { workspace, api, statuses, sync } = harness({ "main.tex": "a" });

  workspace.write("main.tex", "ab");
  await sync.flush();
  sync.stop();

  assert.equal(api.files.get("main.tex"), "ab");
  assert.deepEqual(workspace.dirty(), []);
  assert.deepEqual(statuses, ["dirty", "saving", "synced"]);
});

test("replays a removal as a delete, and tolerates one the server never had", async () => {
  const { workspace, api, sync } = harness({ "main.tex": "a", "notes.tex": "b" });

  workspace.remove("notes.tex");
  workspace.create("draft.tex", "d");
  workspace.remove("draft.tex"); // never reached the server
  await sync.flush();
  sync.stop();

  assert.equal(api.files.has("notes.tex"), false);
  assert.equal(api.files.has("draft.tex"), false);
  assert.equal(sync.pending, false);
});

test("a rename deletes the old path and writes the new one", async () => {
  const { workspace, api, sync } = harness({ "old.tex": "text" });

  workspace.rename("old.tex", "new.tex");
  await sync.flush();
  sync.stop();

  assert.equal(api.files.has("old.tex"), false);
  assert.equal(api.files.get("new.tex"), "text");
});

test("remove then recreate converges on the file existing", async () => {
  const { workspace, api, sync } = harness({ "main.tex": "a", "part.tex": "old" });

  workspace.remove("part.tex");
  workspace.create("part.tex", "new");
  await sync.flush();
  sync.stop();

  assert.equal(api.files.get("part.tex"), "new");
  // Deletes go first, which is the order with that outcome.
  const deleteAt = api.calls.indexOf("delete part.tex");
  const putAt = api.calls.indexOf("put part.tex");
  assert.ok(deleteAt !== -1 && putAt !== -1 && deleteAt < putAt);
});

test("a keystroke during the request leaves the path dirty, then a second round saves it", async () => {
  const { workspace, api, sync } = harness({ "main.tex": "a" });

  workspace.write("main.tex", "ab");
  const racing = api.writeFile.bind(api);
  api.writeFile = async (path, text) => {
    await racing(path, text);
    // Typed while the PUT was in flight.
    if (text === "ab") workspace.write("main.tex", "abc");
  };

  await sync.flush();
  sync.stop();

  // The second round wrote the newer text; nothing believes "ab" was the save.
  assert.equal(api.files.get("main.tex"), "abc");
  assert.deepEqual(workspace.dirty(), []);
});

test("a lapsed session reports signed-out, and retrying after sign-in saves", async () => {
  const { workspace, api, statuses, sync } = harness({ "main.tex": "a" });

  workspace.write("main.tex", "ab");
  api.failWith = 419;
  await sync.flush();
  assert.equal(statuses.at(-1), "signed-out");
  assert.deepEqual(workspace.dirty(), ["main.tex"]);

  api.failWith = null;
  await sync.flush();
  sync.stop();

  assert.equal(api.files.get("main.tex"), "ab");
  assert.equal(statuses.at(-1), "synced");
});

test("a workspace born dirty is pending before anything is typed", () => {
  const workspace = new Workspace();
  workspace.create("main.tex", "seeded");

  const api = new FakeApi();
  const sync = new ProjectSync(workspace, api, () => {}, {
    debounceMs: 3_600_000,
    retryMs: 3_600_000,
  });

  assert.equal(sync.pending, true);
  sync.stop();
});
