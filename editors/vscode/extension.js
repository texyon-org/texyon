"use strict";

const cp = require("child_process");
const path = require("path");
const readline = require("readline");
const vscode = require("vscode");

const { childLogicalPath, sourceFilePath } = require("./path-authority");
const { PreviewPanel } = require("./panel");

const PROTOCOL_VERSION = 1;
let activeSession = null;

class PreviewSession {
  constructor(context, document, configuration, output, status, onClose) {
    this.document = document;
    this.output = output;
    this.status = status;
    this.onClose = onClose;
    this.disposables = [];
    this.ready = false;
    this.initialized = false;
    this.hasFrame = false;
    this.disposed = false;
    this.synctex = false;
    this.nextQuery = 0n;

    const executable = configuration.get("executable", "tectonic");
    const format = configuration.get("format", "latex");
    const args = ["-X", "preview", "--format", format];
    this.output.appendLine(`[preview] spawning ${executable} ${args.join(" ")}`);
    this.setStatus("$(sync~spin) tectonic: starting…");

    this.panel = new PreviewPanel(
      context,
      `Preview: ${path.basename(document.fileName)}`,
      (message) => this.onViewerMessage(message)
    );
    this.panel.onDispose = () => this.dispose(true);
    this.process = cp.spawn(executable, args, {
      cwd: path.dirname(document.fileName),
      stdio: ["pipe", "pipe", "pipe"]
    });
    this.process.on("error", (error) => this.fail(`failed to start: ${error.message}`));
    this.process.on("exit", (code, signal) => this.onExit(code, signal));
    this.process.stderr.on("data", (data) => this.output.append(data.toString()));
    this.lines = readline.createInterface({ input: this.process.stdout });
    this.lines.on("line", (line) => this.onServerLine(line));

    this.disposables.push(vscode.workspace.onDidChangeTextDocument((event) => {
      if (event.contentChanges.length === 0 || this.disposed || !this.initialized) return;
      if (event.document === this.document) {
        this.send({ type: "replace", text: event.document.getText() });
        this.setStatus("$(sync~spin) tectonic: compiling…");
        return;
      }
      const logical = this.childPath(event.document);
      if (logical !== null) {
        this.send({ type: "replace", path: logical, text: event.document.getText() });
        this.setStatus("$(sync~spin) tectonic: compiling…");
      }
    }));
  }

  setStatus(text) {
    this.status.text = text;
    this.status.show();
  }

  childPath(document) {
    if (document.isUntitled || document.uri.scheme !== "file") return null;
    try {
      return childLogicalPath(this.document.fileName, document.fileName);
    } catch (error) {
      this.output.appendLine(`[preview] ignoring child ${document.fileName}: ${error.message}`);
      return null;
    }
  }

  initialize(protocolVersion) {
    const children = [];
    const seen = new Set();
    for (const document of vscode.workspace.textDocuments) {
      if (!document.isDirty || document === this.document) continue;
      const logical = this.childPath(document);
      if (logical !== null && !seen.has(logical)) {
        seen.add(logical);
        children.push({ path: logical, text: document.getText() });
      }
    }
    children.sort((left, right) => left.path.localeCompare(right.path));
    this.send({
      type: "initialize",
      protocol_version: protocolVersion,
      primary_path: this.document.fileName,
      primary_text: this.document.getText(),
      children
    });
    this.initialized = true;
    this.setStatus("$(sync~spin) tectonic: compiling…");
  }

  send(message) {
    if (this.disposed || !this.process || this.process.exitCode !== null || !this.process.stdin.writable) return;
    this.process.stdin.write(`${JSON.stringify(message)}\n`);
  }

  onServerLine(line) {
    let event;
    try {
      event = JSON.parse(line);
    } catch (error) {
      this.fail(`server emitted invalid JSON: ${error.message}`);
      return;
    }
    if (!this.ready) {
      if (event.type !== "ready") {
        this.fail(`expected ready, received ${event.type || "unknown event"}`);
        return;
      }
      if (event.protocol_version !== PROTOCOL_VERSION ||
          !Array.isArray(event.capabilities) || !event.capabilities.includes("svg")) {
        this.fail(`incompatible preview protocol ${event.protocol_version}`);
        return;
      }
      this.ready = true;
      this.synctex = event.capabilities.includes("synctex");
      this.output.appendLine(`[preview] ready; bundle ${event.bundle_digest}`);
      this.initialize(event.protocol_version);
      return;
    }

    switch (event.type) {
      case "update":
        this.hasFrame = true;
        this.panel.postUpdate(event);
        this.setStatus(event.outcome === "error"
          ? "$(warning) tectonic: source error (kept coherent preview)"
          : `$(check) tectonic: ${event.page_count} page${event.page_count === 1 ? "" : "s"}`);
        break;
      case "cancelled":
        break;
      case "stopped":
        this.setStatus("$(debug-pause) tectonic: stopped; edit to resume");
        break;
      case "aborted":
        this.output.appendLine(`[preview] compile aborted:\n${event.log_tail || ""}`);
        this.setStatus("$(warning) tectonic: source error (kept last preview)");
        break;
      case "rejected":
        this.output.appendLine(`[preview] request rejected: ${event.reason}`);
        this.setStatus("$(warning) tectonic: edit rejected");
        break;
      case "forward_search_result":
        this.panel.showForwardSearch(event.result || null);
        break;
      case "backward_search_result":
        if (event.result) this.revealSource(event.result);
        break;
      case "fatal":
        this.fail(`server session failed: ${event.reason}`);
        break;
      default:
        this.fail(`unknown server event ${event.type}`);
    }
  }

  onViewerMessage(message) {
    if (!message || typeof message.type !== "string") return;
    if (message.type === "ready") {
      if (this.hasFrame) this.send({ type: "resync" });
    } else if (message.type === "viewport") {
      this.send({ type: "viewport", first: message.first, last: message.last });
    } else if (message.type === "painted") {
      this.send({ type: "painted", frame_id: message.frame_id, paint_ms: message.paint_ms });
    } else if (message.type === "backward_search" && this.synctex) {
      this.send({
        type: "backward_search",
        query_id: this.queryId(),
        page_index: message.page_index,
        h_bp: message.h_bp,
        v_bp: message.v_bp
      });
    } else if (message.type === "viewer_error") {
      this.fail(`webview rejected a frame: ${message.message}`);
    }
  }

  queryId() {
    this.nextQuery = this.nextQuery === ((1n << 64n) - 1n) ? 1n : this.nextQuery + 1n;
    return this.nextQuery.toString();
  }

  forwardSearch(editor) {
    if (!this.initialized || !this.synctex || !editor) return false;
    let logical;
    if (editor.document === this.document) logical = this.document.fileName;
    else logical = this.childPath(editor.document);
    if (logical === null) return false;
    this.send({
      type: "forward_search",
      query_id: this.queryId(),
      path: logical,
      line: editor.selection.active.line + 1
    });
    this.panel.reveal();
    return true;
  }

  async revealSource(result) {
    try {
      const fileName = sourceFilePath(this.document.fileName, result.path);
      const document = await vscode.workspace.openTextDocument(vscode.Uri.file(fileName));
      const line = Math.max(0, Math.min(document.lineCount - 1, result.line - 1));
      const position = new vscode.Position(line, 0);
      const editor = await vscode.window.showTextDocument(document, {
        viewColumn: vscode.ViewColumn.One,
        preserveFocus: false,
        preview: false
      });
      editor.selection = new vscode.Selection(position, position);
      editor.revealRange(
        new vscode.Range(position, position),
        vscode.TextEditorRevealType.InCenterIfOutsideViewport
      );
    } catch (error) {
      this.output.appendLine(`[preview] could not reveal source: ${error.message}`);
    }
  }

  cancel() {
    if (this.initialized) this.send({ type: "cancel" });
  }

  fail(message) {
    if (this.disposed) return;
    this.output.appendLine(`[preview] ${message}`);
    this.panel.showError(message);
    this.setStatus("$(error) tectonic: preview failed");
    vscode.window.showErrorMessage(`Tectonic preview: ${message}`);
    this.dispose(false, true);
  }

  onExit(code, signal) {
    if (this.disposed) return;
    const detail = signal ? `signal ${signal}` : `exit ${code}`;
    this.output.appendLine(`[preview] server stopped (${detail})`);
    this.panel.showError(`Preview server stopped (${detail}). Start a new preview to recover.`);
    this.setStatus(code === 0 ? "tectonic: stopped" : "$(error) tectonic: server died");
    this.dispose(false, true);
  }

  dispose(sendShutdown, retainPanel = false) {
    if (this.disposed) return;
    this.disposed = true;
    if (sendShutdown) this.sendRawShutdown();
    if (sendShutdown) this.setStatus("tectonic: stopped");
    if (this.lines) this.lines.close();
    for (const disposable of this.disposables.splice(0)) disposable.dispose();
    if (!retainPanel) this.panel.dispose();
    if (this.process && this.process.exitCode === null) {
      const process = this.process;
      setTimeout(() => {
        if (process.exitCode === null) process.kill();
      }, 1500).unref();
    }
    this.onClose(this);
  }

  sendRawShutdown() {
    if (this.process && this.process.exitCode === null && this.process.stdin.writable) {
      this.process.stdin.end(`${JSON.stringify({ type: "shutdown" })}\n`);
    }
  }
}

function activate(context) {
  const output = vscode.window.createOutputChannel("Tectonic Live Preview");
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 20);
  status.command = "tectonicPreview.start";
  context.subscriptions.push(output, status);

  context.subscriptions.push(vscode.commands.registerCommand("tectonicPreview.start", () => {
    const document = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document;
    if (!document || document.isUntitled || document.uri.scheme !== "file") {
      vscode.window.showErrorMessage("Open a saved TeX document before starting live preview.");
      return;
    }
    if (activeSession) activeSession.dispose(true);
    const configuration = vscode.workspace.getConfiguration("tectonicPreview");
    activeSession = new PreviewSession(context, document, configuration, output, status, (closed) => {
      if (activeSession === closed) activeSession = null;
    });
  }));
  context.subscriptions.push(vscode.commands.registerCommand("tectonicPreview.stop", () => {
    if (activeSession) activeSession.dispose(true);
  }));
  context.subscriptions.push(vscode.commands.registerCommand("tectonicPreview.cancel", () => {
    if (activeSession) activeSession.cancel();
  }));
  context.subscriptions.push(vscode.commands.registerCommand("tectonicPreview.forwardSearch", () => {
    const editor = vscode.window.activeTextEditor;
    if (!activeSession || !activeSession.forwardSearch(editor)) {
      vscode.window.showInformationMessage("Start a SyncTeX-capable preview and place the cursor in one of its TeX sources.");
    }
  }));
}

function deactivate() {
  if (activeSession) activeSession.dispose(true);
}

module.exports = { activate, deactivate };
