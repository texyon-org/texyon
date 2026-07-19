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
    getText: () => "primary from editor"
  };
  const child = {
    fileName: childPath,
    isDirty: true,
    isUntitled: false,
    uri: { scheme: "file" },
    getText: () => "child from editor"
  };

  const commands = new Map();
  const errors = [];
  const panels = [];
  let changeListener = null;
  const output = { text: "", append(value) { this.text += value; }, appendLine(value) { this.text += `${value}\n`; }, dispose() {} };
  const status = { text: "", show() {}, dispose() {} };
  const vscode = {
    StatusBarAlignment: { Left: 1 },
    ViewColumn: { Beside: 2 },
    Uri: {
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
      onDidChangeTextDocument(callback) {
        changeListener = callback;
        return { dispose() { changeListener = null; } };
      }
    },
    window: {
      activeTextEditor: { document: primary },
      createOutputChannel: () => output,
      createStatusBarItem: () => status,
      showErrorMessage: (message) => errors.push(message),
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
    }
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

  fakeProcess.event({ type: "ready", protocol_version: 1, capabilities: ["svg"], bundle_digest: "abc" });
  await tick();
  assert.equal(fakeProcess.requests[0].type, "initialize");
  assert.equal(fakeProcess.requests[0].primary_text, "primary from editor");
  assert.deepEqual(fakeProcess.requests[0].children, [{ path: "chapter.tex", text: "child from editor" }]);

  fakeProcess.event({ type: "update", frame_id: "1", source_revision: "0", outcome: "complete", page_count: 1 });
  await tick();
  assert.equal(panels[0].posted.at(-1).event.frame_id, "1");
  panels[0].receive({ type: "ready" });
  assert.equal(fakeProcess.requests.at(-1).type, "resync");
  panels[0].receive({ type: "viewport", first: 0, last: 0 });
  panels[0].receive({ type: "painted", frame_id: "1", paint_ms: 4.5 });
  assert.equal(fakeProcess.requests.at(-1).type, "painted");

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
