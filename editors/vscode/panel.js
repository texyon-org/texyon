"use strict";

const vscode = require("vscode");

/** The webview that shows the pages. */
class PreviewPanel {
  constructor(context, title, onMessage) {
    this.disposed = false;
    this.onDispose = null;
    this.mediaRoot = vscode.Uri.joinPath(context.extensionUri, "media");
    this.panel = vscode.window.createWebviewPanel(
      "texyon",
      title,
      { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [this.mediaRoot]
      }
    );
    this.panel.webview.html = this.html(this.panel.webview);
    this.panel.webview.onDidReceiveMessage(onMessage);
    this.panel.onDidDispose(() => {
      this.disposed = true;
      if (this.onDispose) this.onDispose();
    });
  }

  post(message) {
    if (!this.disposed) void this.panel.webview.postMessage(message);
  }

  reveal() {
    if (!this.disposed) this.panel.reveal(vscode.ViewColumn.Beside, true);
  }

  dispose() {
    if (!this.disposed) this.panel.dispose();
  }

  html(webview) {
    const media = (name) => webview.asWebviewUri(vscode.Uri.joinPath(this.mediaRoot, name));
    const csp = [
      "default-src 'none'",
      `script-src ${webview.cspSource}`,
      `style-src ${webview.cspSource}`
    ].join("; ");
    return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" href="${media("viewer.css")}">
  <title>TeXyoN Preview</title>
</head>
<body>
  <div id="toolbar" role="toolbar" aria-label="Preview zoom">
    <button id="zoom-out" title="Zoom out">−</button>
    <button id="zoom-reset" title="Fit width">Fit</button>
    <button id="zoom-in" title="Zoom in">+</button>
    <span id="tail" hidden>Typesetting the rest…</span>
    <span id="error" hidden></span>
  </div>
  <main id="pages"><p id="empty">Waiting for the first compile…</p></main>
  <script src="${media("viewer.js")}"></script>
</body>
</html>`;
  }
}

module.exports = { PreviewPanel };
