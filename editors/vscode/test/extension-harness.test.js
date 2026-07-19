"use strict";

const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const fs = require("node:fs");
const Module = require("node:module");
const os = require("node:os");
const path = require("node:path");
const { PassThrough } = require("node:stream");
const test = require("node:test");

const tick = () => new Promise((resolve) => setImmediate(resolve));

class FakeProcess extends EventEmitter {
  constructor() {
    super();
    this.stdout = new PassThrough();
    this.stderr = new PassThrough();
    this.exitCode = null;
    this.requests = [];
    this.stdin = {
      writable: true,
      write: (text) => {
        this.requests.push(JSON.parse(text));
        return true;
      },
      end: (text) => {
        if (text) this.requests.push(JSON.parse(text));
        this.stdin.writable = false;
      }
    };
  }

  event(value) {
    this.stdout.write(`${JSON.stringify(value)}\n`);
  }

  kill() {
    this.exitCode = null;
  }
}

test("extension harness initializes dirty children, relays edits, resyncs, and retains last frame on death", async (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tectonic-extension-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const primaryPath = path.join(root, "main.tex");
  const childPath = path.join(root, "chapter.tex");
  fs.writeFileSync(primaryPath, "primary on disk");
  fs.writeFileSync(childPath, "child on disk");
  const primary = {
    fileName: primaryPath,
    isDirty: true,
    isUntitled: false,
    uri: { scheme: "file" },
    getText: () => "primary from editor",
    lineCount: 8
  };
  const child = {
    fileName: childPath,
    isDirty: true,
    isUntitled: false,
    uri: { scheme: "file" },
    getText: () => "child from editor",
    lineCount: 6
  };

  const commands = new Map();
  const errors = [];
  const panels = [];
  const shownEditors = [];
  let changeListener = null;
  const output = { text: "", append(value) { this.text += value; }, appendLine(value) { this.text += `${value}\n`; }, dispose() {} };
  const status = { text: "", show() {}, dispose() {} };
  const vscode = {
    StatusBarAlignment: { Left: 1 },
    ViewColumn: { Beside: 2 },
    Uri: {
      file(fileName) {
        return { scheme: "file", fsPath: fileName };
      },
      joinPath(base, ...parts) {
        const text = [String(base), ...parts].join("/");
        return { toString: () => text };
      }
    },
    commands: {
      registerCommand(name, callback) {
        commands.set(name, callback);
        return { dispose() {} };
      }
    },
    workspace: {
      textDocuments: [primary, child],
      getConfiguration: () => ({ get: (name, fallback) => name === "executable" ? "fake-tectonic" : fallback }),
      openTextDocument: async (uri) => uri.fsPath === childPath ? child : primary,
      onDidChangeTextDocument(callback) {
        changeListener = callback;
        return { dispose() { changeListener = null; } };
      }
    },
    window: {
      activeTextEditor: { document: primary, selection: { active: { line: 2 } } },
      createOutputChannel: () => output,
      createStatusBarItem: () => status,
      showErrorMessage: (message) => errors.push(message),
      showInformationMessage: (message) => errors.push(message),
      showTextDocument: async (document) => {
        const editor = { document, selection: null, revealRange() {} };
        shownEditors.push(editor);
        return editor;
      },
      createWebviewPanel() {
        const messageEmitter = new EventEmitter();
        const disposeEmitter = new EventEmitter();
        const posted = [];
        const panel = {
          posted,
          disposed: false,
          webview: {
            cspSource: "vscode-webview:",
            html: "",
            asWebviewUri: (uri) => uri,
            postMessage: (message) => { posted.push(message); },
            onDidReceiveMessage: (callback) => {
              messageEmitter.on("message", callback);
              return { dispose() {} };
            }
          },
          onDidDispose: (callback) => {
            disposeEmitter.on("dispose", callback);
            return { dispose() {} };
          },
          receive: (message) => messageEmitter.emit("message", message),
          reveal() {},
          dispose() {
            if (!this.disposed) {
              this.disposed = true;
              disposeEmitter.emit("dispose");
            }
          }
        };
        panels.push(panel);
        return panel;
      }
    },
    Position: class Position {
      constructor(line, character) { this.line = line; this.character = character; }
    },
    Selection: class Selection {
      constructor(anchor, active) { this.anchor = anchor; this.active = active; }
    },
    Range: class Range {
      constructor(start, end) { this.start = start; this.end = end; }
    },
    TextEditorRevealType: { InCenterIfOutsideViewport: 1 }
  };

  const fakeProcess = new FakeProcess();
  const childProcess = require("node:child_process");
  const originalSpawn = childProcess.spawn;
  childProcess.spawn = () => fakeProcess;
  const originalLoad = Module._load;
  Module._load = function (request, parent, isMain) {
    if (request === "vscode") return vscode;
    return originalLoad.call(this, request, parent, isMain);
  };
  t.after(() => {
    childProcess.spawn = originalSpawn;
    Module._load = originalLoad;
  });

  const extensionPath = require.resolve("../extension");
  const panelPath = require.resolve("../panel");
  delete require.cache[extensionPath];
  delete require.cache[panelPath];
  const extension = require("../extension");
  const context = { extensionUri: "/extension", subscriptions: [] };
  extension.activate(context);
  commands.get("tectonicPreview.start")();
  assert.equal(panels.length, 1);
  const html = panels[0].webview.html;
  assert.match(html, /default-src 'none'/);
  assert.match(html, /script-src vscode-webview:/);
  assert.match(html, /style-src vscode-webview:/);
  assert.match(html, /img-src data:/);
  assert.doesNotMatch(html, /unsafe-inline|unsafe-eval|https?:/);

  fakeProcess.event({ type: "ready", protocol_version: 1, capabilities: ["svg", "synctex"], bundle_digest: "abc" });
  await tick();
  assert.equal(fakeProcess.requests[0].type, "initialize");
  assert.equal(fakeProcess.requests[0].primary_text, "primary from editor");
  assert.deepEqual(fakeProcess.requests[0].children, [{ path: "chapter.tex", text: "child from editor" }]);

  commands.get("tectonicPreview.forwardSearch")();
  assert.deepEqual(fakeProcess.requests.at(-1), {
    type: "forward_search",
    query_id: "1",
    path: primaryPath,
    line: 3
  });
  panels[0].receive({ type: "backward_search", page_index: 2, h_bp: 30, v_bp: 40 });
  assert.deepEqual(fakeProcess.requests.at(-1), {
    type: "backward_search",
    query_id: "2",
    page_index: 2,
    h_bp: 30,
    v_bp: 40
  });
  fakeProcess.event({
    type: "forward_search_result",
    query_id: "1",
    result: { page_index: 0, h_bp: 72, v_bp: 80, width_bp: 10, height_bp: 8, depth_bp: 2 }
  });
  await tick();
  assert.equal(panels[0].posted.at(-1).type, "forward_search");
  fakeProcess.event({
    type: "backward_search_result",
    query_id: "2",
    result: { path: "chapter.tex", line: 4 }
  });
  await tick();
  await tick();
  assert.equal(shownEditors.at(-1).document, child);
  assert.equal(shownEditors.at(-1).selection.active.line, 3);

  fakeProcess.event({
    type: "update",
    frame_id: "1",
    source_revision: "0",
    applied_request_seq: "0",
    phase: "foreground",
    outcome: "complete",
    page_count: 1,
    tail_pending: false,
    unsupported_pages: [{ page_index: 0, operations: { "missing-image-resource": 1 } }]
  });
  await tick();
  assert.equal(panels[0].posted.at(-1).event.frame_id, "1");
  assert.match(status.text, /unsupported content/);
  assert.match(output.text, /missing-image-resource/);
  assert.match(output.text, /end-to-end/);
  panels[0].receive({ type: "ready" });
  assert.equal(fakeProcess.requests.at(-1).type, "resync");
  panels[0].receive({ type: "viewport", first: 0, last: 0 });
  panels[0].receive({ type: "painted", frame_id: "1", paint_ms: 4.5 });
  assert.equal(fakeProcess.requests.at(-1).type, "painted");
  assert.match(output.text, /painted in 4\.50 ms/);
  const requestCount = fakeProcess.requests.length;
  panels[0].receive({ type: "painted", frame_id: "1", paint_ms: -1 });
  assert.equal(fakeProcess.requests.length, requestCount);

  changeListener({ document: child, contentChanges: [{}] });
  assert.deepEqual(fakeProcess.requests.at(-1), {
    type: "replace",
    path: "chapter.tex",
    text: "child from editor"
  });
  fakeProcess.event({ type: "aborted", log_tail: "temporary TeX error" });
  await tick();
  assert.equal(panels[0].disposed, false);

  fakeProcess.exitCode = 1;
  fakeProcess.emit("exit", 1, null);
  assert.equal(panels[0].disposed, false);
  assert.match(panels[0].posted.at(-1).message, /server stopped/i);
  assert.equal(errors.length, 0);
});
