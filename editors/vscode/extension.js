"use strict";

const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const { NativeEngine } = require("./engine");
const { CompileLoop } = require("./loop");
const { PreviewPanel } = require("./panel");
const { childLogicalPath, sourceFilePath } = require("./path-authority");
const { SourceTracker } = require("./sources");

/** A server that dies sooner than this after starting is not restarted. */
const RESTART_WINDOW_MS = 10_000;

let activeSession = null;

class PreviewSession {
  constructor(context, document, output, status, diagnostics, onClose) {
    this.context = context;
    this.document = document;
    this.output = output;
    this.status = status;
    this.diagnostics = diagnostics;
    this.onClose = onClose;
    this.root = path.dirname(document.fileName);
    this.primary = path.basename(document.fileName);
    this.disposed = false;
    this.disposables = [];
    this.view = { viewport: [0, 0], scale: 2 };
    this.sources = new SourceTracker();

    this.panel = new PreviewPanel(context, `Preview: ${this.primary}`, (message) => this.onViewerMessage(message));
    this.panel.onDispose = () => this.dispose();

    this.track(document);
    for (const open of vscode.workspace.textDocuments) this.track(open);

    this.disposables.push(
      vscode.workspace.onDidOpenTextDocument((opened) => {
        if (this.track(opened)) this.loop.schedule();
      }),
      vscode.workspace.onDidChangeTextDocument((event) => {
        const logical = this.logicalPath(event.document);
        if (logical === null || event.contentChanges.length === 0) return;
        this.sources.change(logical, event.contentChanges);
        this.loop.schedule();
      })
    );
    this.start();
  }

  start() {
    const configuration = vscode.workspace.getConfiguration("texyon");
    const executable = resolveExecutable(this.context, configuration);
    this.startedAt = Date.now();
    this.output.appendLine(`[preview] starting ${executable} for ${this.document.fileName}`);
    this.setStatus("$(sync~spin) TeXyoN: starting…");
    this.engine = new NativeEngine(executable, {
      cwd: this.root,
      onStderr: (text) => this.output.append(text),
      onExit: (code, signal) => this.onExit(code, signal)
    });
    this.sources.reset();
    this.loop = new CompileLoop(this.engine, this.sources, () => this.view, {
      compiled: (report) => this.compiled(report),
      failed: (message) => this.failed(message),
      busy: (running) => {
        if (running) this.setStatus("$(sync~spin) TeXyoN: compiling…");
      }
    });
    const opened = this.engine.request({
      op: "open",
      format: configuration.get("format", "latex"),
      primary: this.primary,
      root: this.root,
      raster: true
    });
    opened.then((response) => {
      if (response.status !== "ok") this.fatal(response.message || "could not open the preview session");
      else this.loop.schedule();
    }, (error) => this.fatal(error.message));
  }

  /** Track a document as a project source. Returns whether it is one. */
  track(document) {
    const logical = this.logicalPath(document);
    if (logical === null) return false;
    this.sources.track(logical, () => document.getText());
    return true;
  }

  logicalPath(document) {
    if (document.isUntitled || document.uri.scheme !== "file") return null;
    if (document.fileName === this.document.fileName) return this.primary;
    try {
      return childLogicalPath(this.document.fileName, document.fileName);
    } catch (error) {
      this.output.appendLine(`[preview] ignoring ${document.fileName}: ${error.message}`);
      return null;
    }
  }

  compiled(report) {
    this.panel.post({ type: "compiled", report: withOwnedPixels(report), scale: this.view.scale });
    if (report.published || report.aborted) this.showDiagnostics(report.diagnostics);
    if (report.aborted) this.output.appendLine(`[preview] run abandoned: ${report.aborted}`);
    for (const message of report.messages) this.output.appendLine(`[preview] ${message}`);
    const errors = report.diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
    this.setStatus(errors > 0
      ? `$(error) TeXyoN: ${errors} error${errors === 1 ? "" : "s"}`
      : `$(check) TeXyoN: ${report.pageCount} page${report.pageCount === 1 ? "" : "s"}`);
    this.output.appendLine(
      `[preview] ${report.published ? "frame" : "no frame"}: tex ${report.texMs.toFixed(1)} ms, ` +
      `render ${report.renderMs.toFixed(1)} ms, ${report.pagesRendered} page(s) drawn`
    );
  }

  failed(message) {
    this.output.appendLine(`[preview] ${message}`);
    this.panel.post({ type: "error", message });
    this.setStatus("$(warning) TeXyoN: preview failed, edit to retry");
  }

  showDiagnostics(diagnostics) {
    const byFile = new Map();
    for (const found of diagnostics) {
      if (found.severity === "info" || found.path === null) continue;
      let file;
      try {
        file = sourceFilePath(this.document.fileName, found.path.replace(/^\.\//, ""));
      } catch {
        continue;
      }
      // Classes and packages come from the bundle, not from the project.
      if (!fs.existsSync(file)) continue;
      const line = Math.max(0, (found.line || 1) - 1);
      const entry = new vscode.Diagnostic(
        new vscode.Range(line, 0, line, Number.MAX_SAFE_INTEGER),
        found.message,
        found.severity === "error" ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning
      );
      entry.source = "TeXyoN";
      if (!byFile.has(file)) byFile.set(file, []);
      byFile.get(file).push(entry);
    }
    this.diagnostics.clear();
    for (const [file, entries] of byFile) this.diagnostics.set(vscode.Uri.file(file), entries);
  }

  onViewerMessage(message) {
    if (!message || typeof message.type !== "string") return;
    if (message.type === "view") {
      const [first, last] = message.viewport;
      if (!Number.isSafeInteger(first) || !Number.isSafeInteger(last) || !(message.scale > 0)) return;
      this.view = { viewport: [first, last], scale: message.scale };
    } else if (message.type === "raster") {
      void this.raster(message.pages, message.scale);
    } else if (message.type === "backward_search") {
      void this.backwardSearch(message);
    } else if (message.type === "open_uri") {
      openLink(message.uri);
    }
  }

  async raster(pages, scale) {
    if (!Array.isArray(pages) || !pages.every(Number.isSafeInteger) || !(scale > 0)) return;
    try {
      const response = await this.engine.request({ op: "raster", pages, scale });
      if (response.status !== "rastered") throw new Error(response.message);
      this.panel.post({ type: "rastered", pages: response.pages.map(ownPixels), scale });
    } catch {
      this.panel.post({ type: "raster_failed", pages });
    }
  }

  async forwardSearch(editor) {
    const logical = this.logicalPath(editor.document);
    if (logical === null) return false;
    const response = await this.engine.request({
      op: "forwardSearch",
      path: logical,
      line: editor.selection.active.line + 1
    });
    if (response.status === "located") this.panel.post({ type: "located", rect: response.rect });
    this.panel.reveal();
    return true;
  }

  async backwardSearch({ pageIndex, hBp, vBp }) {
    if (![pageIndex, hBp, vBp].every(Number.isFinite)) return;
    const response = await this.engine.request({ op: "backwardSearch", pageIndex, hBp, vBp });
    if (response.status !== "located" || !response.source) return;
    try {
      const file = sourceFilePath(this.document.fileName, response.source.path);
      const document = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
      const line = Math.max(0, Math.min(document.lineCount - 1, response.source.line - 1));
      const position = new vscode.Position(line, 0);
      const editor = await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.One, preview: false });
      editor.selection = new vscode.Selection(position, position);
      editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
    } catch (error) {
      this.output.appendLine(`[preview] could not reveal source: ${error.message}`);
    }
  }

  async exportPdf() {
    const target = path.join(this.root, `${path.parse(this.primary).name}.pdf`);
    this.setStatus("$(sync~spin) TeXyoN: exporting…");
    const response = await this.loop.settled(() => this.engine.request({ op: "export" }));
    if (response.status !== "exported") throw new Error(response.message || "the export failed");
    await fs.promises.writeFile(target, response.data);
    this.setStatus("$(check) TeXyoN: exported");
    return target;
  }

  cancel() {
    this.engine.cancel();
  }

  setStatus(text) {
    this.status.text = text;
    this.status.show();
  }

  onExit(code, signal) {
    if (this.disposed) return;
    this.loop.stop();
    const detail = signal ? `signal ${signal}` : `exit code ${code}`;
    this.output.appendLine(`[preview] server stopped (${detail})`);
    if (Date.now() - this.startedAt > RESTART_WINDOW_MS) {
      this.start();
      return;
    }
    this.fatal(`the preview server stopped (${detail})`);
  }

  fatal(message) {
    if (this.disposed) return;
    this.failed(message);
    this.setStatus("$(error) TeXyoN: preview stopped");
    vscode.window.showErrorMessage(`TeXyoN preview: ${message}`);
    this.dispose(true);
  }

  dispose(retainPanel = false) {
    if (this.disposed) return;
    this.disposed = true;
    this.loop.stop();
    void this.engine.close();
    for (const disposable of this.disposables.splice(0)) disposable.dispose();
    this.diagnostics.clear();
    if (!retainPanel) this.panel.dispose();
    this.onClose(this);
  }
}

/** Copy pixels out of the frame they were decoded from, so the webview gets only them. */
function ownPixels(page) {
  return { index: page.index, data: new Uint8Array(page.data) };
}

function withOwnedPixels(report) {
  return { ...report, rasters: report.rasters.map(ownPixels) };
}

function openLink(uri) {
  if (typeof uri !== "string") return;
  const parsed = vscode.Uri.parse(uri, true);
  if (["http", "https", "mailto"].includes(parsed.scheme)) void vscode.env.openExternal(parsed);
}

function resolveExecutable(context, configuration) {
  const configured = configuration.get("executable", "");
  if (configured) return configured;
  const bundled = context.asAbsolutePath(path.join("bin", process.platform === "win32" ? "texyon.exe" : "texyon"));
  if (!fs.existsSync(bundled)) return "texyon";
  // Some VSIX extractors drop the executable bit.
  if (process.platform !== "win32") {
    try {
      fs.accessSync(bundled, fs.constants.X_OK);
    } catch {
      fs.chmodSync(bundled, 0o755);
    }
  }
  return bundled;
}

function activate(context) {
  const output = vscode.window.createOutputChannel("TeXyoN Live Preview");
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 20);
  const diagnostics = vscode.languages.createDiagnosticCollection("texyon");
  status.command = "texyon.start";
  context.subscriptions.push(output, status, diagnostics);

  const command = (name, run) => context.subscriptions.push(vscode.commands.registerCommand(name, run));

  command("texyon.start", () => {
    const document = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document;
    if (!document || document.isUntitled || document.uri.scheme !== "file") {
      vscode.window.showErrorMessage("Open a saved TeX document before starting the live preview.");
      return;
    }
    if (activeSession) activeSession.dispose();
    activeSession = new PreviewSession(context, document, output, status, diagnostics, (closed) => {
      if (activeSession === closed) activeSession = null;
    });
  });
  command("texyon.stop", () => {
    if (activeSession) activeSession.dispose();
    status.text = "TeXyoN: stopped";
  });
  command("texyon.cancel", () => {
    if (activeSession) activeSession.cancel();
  });
  command("texyon.forwardSearch", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!activeSession || !editor || !(await activeSession.forwardSearch(editor))) {
      vscode.window.showInformationMessage("Start a live preview and place the cursor in one of its TeX sources.");
    }
  });
  command("texyon.exportPdf", async () => {
    if (!activeSession) {
      vscode.window.showInformationMessage("Start a live preview first.");
      return;
    }
    try {
      const target = await activeSession.exportPdf();
      vscode.window.showInformationMessage(`Exported ${path.basename(target)}.`);
    } catch (error) {
      vscode.window.showErrorMessage(`TeXyoN export: ${error.message}`);
    }
  });
}

function deactivate() {
  if (activeSession) activeSession.dispose();
}

module.exports = { activate, deactivate };
