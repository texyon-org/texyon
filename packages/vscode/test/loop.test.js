"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");

const { CompileLoop, TAIL_IDLE_MS } = require("../loop");
const { SourceTracker } = require("../sources");

const tick = () => new Promise((resolve) => setImmediate(resolve));
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** An engine whose replies the test releases one at a time. */
class FakeEngine {
  constructor() {
    this.sent = [];
    this.cancels = 0;
    this.pending = [];
  }

  request(message) {
    this.sent.push(message);
    return new Promise((resolve) => this.pending.push(resolve));
  }

  cancel() {
    this.cancels += 1;
  }

  reply(response) {
    this.pending.shift()(response);
  }
}

const compiled = (tailPending) => ({ status: "compiled", published: true, tailPending });

function setup() {
  const engine = new FakeEngine();
  const sources = new SourceTracker();
  let text = "a";
  sources.track("main.tex", () => text);
  const reports = [];
  const loop = new CompileLoop(engine, sources, () => ({ viewport: [0, 0], scale: 2 }), {
    compiled: (report) => reports.push(report),
    failed: (message) => reports.push(message),
    busy: () => {}
  });
  const edit = (next) => {
    text = next;
    sources.change("main.tex", [{ rangeOffset: 0, rangeLength: 1, text: next }]);
    loop.schedule();
  };
  return { engine, loop, reports, edit };
}

test("edits during a compile never cancel it and merge into one follow-up", async () => {
  const { engine, loop, edit } = setup();
  loop.schedule();
  assert.equal(engine.sent.length, 1);
  edit("b");
  edit("c");
  assert.equal(engine.cancels, 0);
  assert.equal(engine.sent.length, 1);

  engine.reply(compiled(false));
  await tick();
  assert.equal(engine.sent.length, 2);
  assert.equal(engine.sent[1].op, "compile");
  assert.deepEqual(engine.sent[1].texts, { "main.tex": "c" });
  loop.stop();
});

test("the tail waits for a pause and an edit cancels it", async () => {
  const { engine, loop, edit } = setup();
  loop.schedule();
  engine.reply(compiled(true));
  await tick();
  assert.equal(engine.sent.length, 1);

  await sleep(TAIL_IDLE_MS + 30);
  assert.equal(engine.sent[1].op, "continueTail");
  edit("b");
  assert.equal(engine.cancels, 1);

  engine.reply({ status: "compiled", published: false, tailPending: true });
  await tick();
  assert.equal(engine.sent[2].op, "compile");
  loop.stop();
});

test("an export settles the tail first and edits do not cancel that", async () => {
  const { engine, loop, edit } = setup();
  loop.schedule();
  engine.reply(compiled(true));
  await tick();

  const exported = loop.settled(() => engine.request({ op: "export" }));
  await tick();
  assert.equal(engine.sent[1].op, "continueTail");
  edit("b");
  assert.equal(engine.cancels, 0);
  engine.reply(compiled(false));
  await tick();
  assert.equal(engine.sent[2].op, "compile");
  engine.reply(compiled(false));
  await tick();
  await tick();
  assert.equal(engine.sent[3].op, "export");
  engine.reply({ status: "exported", data: new Uint8Array() });
  assert.equal((await exported).status, "exported");
  loop.stop();
});

test("a failed compile resends every text but waits for the next edit", async () => {
  const { engine, loop, reports, edit } = setup();
  loop.schedule();
  engine.reply(compiled(false));
  await tick();
  edit("b");
  assert.deepEqual(Object.keys(engine.sent[1].edits), ["main.tex"]);
  engine.reply({ status: "error", message: "bad source" });
  await tick();
  assert.deepEqual(reports.slice(1), ["bad source"]);
  assert.equal(engine.sent.length, 2);

  edit("c");
  assert.deepEqual(engine.sent[2].edits, {});
  assert.deepEqual(engine.sent[2].texts, { "main.tex": "c" });
  loop.stop();
});
