"use strict";

/** Typing pause before the pages past the viewport are typeset. */
const TAIL_IDLE_MS = 200;

/**
 * Keeps one compile in flight and the engine as close to the editor as it
 * can get.
 *
 * A compile always runs to completion, so fast typing still shows a frame
 * per compile. Edits made meanwhile merge into the next one. Typesetting
 * the pages past the viewport waits for a pause and is cancelled by the
 * next edit, since nobody is looking at them while typing.
 */
class CompileLoop {
  /**
   * @param engine `request(message)` and `cancel()`, see engine.js.
   * @param sources A SourceTracker.
   * @param view Returns `{ viewport: [first, last], scale }`.
   * @param sink `compiled(report)`, `failed(message)` and `busy(running)`.
   */
  constructor(engine, sources, view, sink) {
    this.engine = engine;
    this.sources = sources;
    this.view = view;
    this.sink = sink;
    this.running = null;
    this.wanted = true;
    this.tailPending = false;
    this.tailTimer = null;
    this.stopped = false;
    this.idleWaiters = [];
  }

  /** The sources or the viewport changed. */
  schedule() {
    this.wanted = true;
    if (this.running === "tail") this.engine.cancel();
    this.pump();
  }

  stop() {
    this.stopped = true;
    clearTimeout(this.tailTimer);
  }

  /** Resolve once nothing runs and nothing is wanted. */
  idle() {
    if (!this.running && !this.wanted) return Promise.resolve();
    return new Promise((resolve) => this.idleWaiters.push(resolve));
  }

  /** Typeset the rest of the document, then run `operation` before any edit can. */
  async settled(operation) {
    for (;;) {
      await this.idle();
      // An edit can start a compile between the idle signal and this check.
      if (this.running || this.wanted) continue;
      if (!this.tailPending) break;
      const settled = await this.exclusive(() => this.runTail());
      if (!settled) break;
    }
    return this.exclusive(operation);
  }

  async exclusive(operation) {
    this.running = "exclusive";
    try {
      return await operation();
    } finally {
      this.running = null;
      this.pump();
    }
  }

  pump() {
    if (this.stopped || this.running) return;
    clearTimeout(this.tailTimer);
    if (this.wanted) {
      void this.run("compile", () => this.compile());
    } else if (this.tailPending) {
      this.tailTimer = setTimeout(() => {
        if (!this.running && !this.wanted) void this.run("tail", () => this.runTail());
      }, TAIL_IDLE_MS);
      this.notifyIdle();
    } else {
      this.notifyIdle();
    }
  }

  notifyIdle() {
    for (const resolve of this.idleWaiters.splice(0)) resolve();
  }

  async run(kind, operation) {
    this.running = kind;
    this.sink.busy(true);
    try {
      await operation();
    } finally {
      this.running = null;
      this.sink.busy(false);
      this.pump();
    }
  }

  async compile() {
    this.wanted = false;
    const manifest = this.sources.manifest();
    const { viewport, scale } = this.view();
    let response;
    try {
      response = await this.engine.request({ op: "compile", ...manifest, viewport, scale });
    } catch (error) {
      this.sources.reset();
      this.sink.failed(error.message);
      return;
    }
    if (response.status === "compiled") {
      this.sources.acknowledge(manifest);
      this.tailPending = response.tailPending;
      this.sink.compiled(response);
    } else {
      this.sources.reset();
      this.sink.failed(response.message || `unexpected ${response.status} reply`);
    }
  }

  /** Returns whether the tail is now settled. */
  async runTail() {
    let response;
    try {
      response = await this.engine.request({ op: "continueTail" });
    } catch (error) {
      this.sink.failed(error.message);
      return false;
    }
    if (response.status === "idle") {
      this.tailPending = false;
      return true;
    }
    if (response.status !== "compiled") {
      this.sink.failed(response.message || `unexpected ${response.status} reply`);
      return false;
    }
    this.tailPending = response.tailPending;
    this.sink.compiled(response);
    return !response.tailPending;
  }
}

module.exports = { CompileLoop, TAIL_IDLE_MS };
