"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");

const { SourceTracker } = require("../sources");

function tracker(text = "abc") {
  const sources = new SourceTracker();
  const document = { text };
  sources.track("main.tex", () => document.text);
  return { sources, document };
}

const change = (rangeOffset, rangeLength, text) => ({ rangeOffset, rangeLength, text });

test("a source the engine does not hold is sent whole", () => {
  const { sources } = tracker();
  assert.deepEqual(sources.manifest(), { files: { "main.tex": 1 }, edits: {}, texts: { "main.tex": "abc" } });
});

test("one change to a held source is sent as a splice from its revision", () => {
  const { sources, document } = tracker();
  sources.acknowledge(sources.manifest());
  document.text = "aXc";
  sources.change("main.tex", [change(1, 1, "X")]);
  assert.deepEqual(sources.manifest(), {
    files: { "main.tex": 2 },
    edits: { "main.tex": { base: 1, from: 1, to: 2, insert: "X" } },
    texts: {}
  });
});

test("a change made while a compile is in flight splices onto the revision it sends", () => {
  const { sources, document } = tracker();
  sources.acknowledge(sources.manifest());
  sources.change("main.tex", [change(0, 0, "1")]);
  const inFlight = sources.manifest();
  sources.change("main.tex", [change(1, 0, "2")]);
  sources.acknowledge(inFlight);
  document.text = "12abc";
  assert.deepEqual(sources.manifest().edits, { "main.tex": { base: 2, from: 1, to: 1, insert: "2" } });
});

test("a second unsent change, or several in one event, falls back to the whole text", () => {
  const { sources, document } = tracker();
  sources.acknowledge(sources.manifest());
  sources.change("main.tex", [change(0, 0, "1")]);
  sources.change("main.tex", [change(1, 0, "2")]);
  document.text = "12abc";
  assert.deepEqual(sources.manifest(), { files: { "main.tex": 3 }, edits: {}, texts: { "main.tex": "12abc" } });

  sources.acknowledge(sources.manifest());
  sources.change("main.tex", [change(0, 1, ""), change(2, 0, "z")]);
  assert.deepEqual(Object.keys(sources.manifest().texts), ["main.tex"]);
});

test("after a failure every source is sent whole again", () => {
  const { sources } = tracker();
  sources.acknowledge(sources.manifest());
  assert.equal(sources.stale, false);
  sources.reset();
  assert.equal(sources.stale, true);
  assert.deepEqual(sources.manifest().texts, { "main.tex": "abc" });
});
