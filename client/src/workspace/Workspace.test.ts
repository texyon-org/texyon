import assert from "node:assert/strict";
import { test } from "node:test";

import { Workspace } from "./Workspace.ts";

test("writing the same text again is not a change", () => {
  // Every editor echoes its own value back on the change it just applied. If
  // that counted, the document would recompile on every keystroke that changed
  // nothing at all.
  const workspace = new Workspace({ "main.tex": "hello" });
  const seen: number[] = [];
  workspace.subscribe((change) => seen.push(change.revision));

  assert.equal(workspace.write("main.tex", "hello"), false);
  assert.equal(workspace.revision, 0);
  assert.deepEqual(seen, []);
  assert.deepEqual(workspace.dirty(), []);

  assert.equal(workspace.write("main.tex", "hello there"), true);
  assert.equal(workspace.revision, 1);
  assert.deepEqual(seen, [1]);
  assert.deepEqual(workspace.dirty(), ["main.tex"]);
});

test("rewriting a binary file with equal bytes is not a change", () => {
  const workspace = new Workspace({ "plot.png": new Uint8Array([1, 2, 3]) });
  const seen: number[] = [];
  workspace.subscribe((change) => seen.push(change.revision));

  assert.equal(workspace.write("plot.png", new Uint8Array([1, 2, 3])), false);
  assert.deepEqual(seen, []);
  assert.deepEqual(workspace.dirty(), []);

  assert.equal(workspace.write("plot.png", new Uint8Array([9, 2, 3])), true);
  assert.deepEqual(seen, [1]);
  assert.deepEqual(workspace.dirty(), ["plot.png"]);
});

test("a file can change kinds, text to bytes and back", () => {
  const workspace = new Workspace({ "notes.txt": "text" });

  assert.equal(workspace.write("notes.txt", new Uint8Array([0, 1])), true);
  assert.equal(workspace.write("notes.txt", "text again"), true);
});

test("a snapshot is what the engine's compile takes", () => {
  const workspace = new Workspace({ "main.tex": "a" });
  workspace.create("chapters/one.tex", "b");

  assert.deepEqual(workspace.snapshot(), {
    "main.tex": "a",
    "chapters/one.tex": "b",
  });
});

test("a rename is one change, not a removal and an addition", () => {
  // A listener must never see a moment where the project has neither path:
  // a file tree that rebuilt in between would flicker, and a save that ran
  // there would delete the file.
  const workspace = new Workspace({ "old.tex": "text" });
  const changes: { changed: readonly string[]; removed: readonly string[] }[] = [];
  workspace.subscribe(({ changed, removed }) => changes.push({ changed, removed }));

  assert.equal(workspace.rename("old.tex", "new.tex"), true);

  assert.equal(changes.length, 1);
  assert.deepEqual(changes[0], { changed: ["new.tex"], removed: ["old.tex"] });
  assert.equal(workspace.read("new.tex"), "text");
  assert.equal(workspace.read("old.tex"), null);
});

test("a rename onto an existing path is refused", () => {
  const workspace = new Workspace({ "a.tex": "one", "b.tex": "two" });

  assert.equal(workspace.rename("a.tex", "b.tex"), false);
  assert.equal(workspace.read("b.tex"), "two", "the target must not be overwritten");
  assert.equal(workspace.revision, 0);
});

test("creating a path that exists does not overwrite it", () => {
  const workspace = new Workspace({ "main.tex": "original" });

  assert.equal(workspace.create("main.tex", "replacement"), false);
  assert.equal(workspace.read("main.tex"), "original");
});

test("saving clears only the paths that were saved", () => {
  // A save is of a particular version, and the user goes on typing while it is
  // in flight. Clearing everything would lose the edits made in between.
  const workspace = new Workspace();
  workspace.create("a.tex", "one");
  workspace.create("b.tex", "two");
  assert.deepEqual(workspace.dirty(), ["a.tex", "b.tex"]);

  workspace.markSaved(["a.tex"]);
  assert.deepEqual(workspace.dirty(), ["b.tex"]);
});

test("removing a file drops it from the project and from what is unsaved", () => {
  const workspace = new Workspace();
  workspace.create("scratch.tex", "notes");

  assert.equal(workspace.remove("scratch.tex"), true);
  assert.equal(workspace.has("scratch.tex"), false);
  assert.deepEqual(workspace.dirty(), []);
  assert.equal(workspace.remove("scratch.tex"), false, "removing twice changes nothing");
});

test("a listener that unsubscribes while being told does not disturb the rest", () => {
  const workspace = new Workspace();
  const told: string[] = [];

  const stop = workspace.subscribe(() => {
    told.push("first");
    stop();
  });
  workspace.subscribe(() => told.push("second"));

  workspace.create("a.tex", "x");
  assert.deepEqual(told, ["first", "second"]);

  workspace.create("b.tex", "y");
  assert.deepEqual(told, ["first", "second", "second"]);
});

test("the revision is what tells a late compile that it is stale", () => {
  const workspace = new Workspace({ "main.tex": "one" });

  const compiledAt = workspace.revision;
  workspace.write("main.tex", "two");

  assert.notEqual(workspace.revision, compiledAt);
});
