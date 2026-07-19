"use strict";

const vscode = require("vscode");

class PreviewPanel {
  constructor(context, title, onMessage) {
    this.disposed = false;
    this.onDispose = null;
    this.mediaRoot = vscode.Uri.joinPath(context.extensionUri, "media");
    this.panel = vscode.window.createWebviewPanel(
      "tectonicPreview",
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

  postUpdate(event) {
    if (!this.disposed) this.panel.webview.postMessage({ type: "update", event });
  }

  showForwardSearch(result) {
    if (!this.disposed) this.panel.webview.postMessage({ type: "forward_search", result });
  }

  showError(message) {
    if (!this.disposed) this.panel.webview.postMessage({ type: "session_error", message });
  }

  reveal() {
    if (!this.disposed) this.panel.reveal(vscode.ViewColumn.Beside, true);
  }

  dispose() {
    if (!this.disposed) this.panel.dispose();
  }

  mediaUri(webview, name) {
    return webview.asWebviewUri(vscode.Uri.joinPath(this.mediaRoot, name));
  }

  html(webview) {
    const csp = [
      "default-src 'none'",
      `script-src ${webview.cspSource}`,
      `style-src ${webview.cspSource}`,
      "img-src data:"
    ].join("; ");
    const stateUri = this.mediaUri(webview, "viewer-state.js");
    const viewerUri = this.mediaUri(webview, "viewer.js");
    const styleUri = this.mediaUri(webview, "viewer.css");
    return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" href="${styleUri}">
  <title>Tectonic Preview</title>
</head>
<body>
  <div id="toolbar" role="toolbar" aria-label="Preview zoom">
    <button id="zoom-out" title="Zoom out">−</button>
    <button id="zoom-reset" title="Fit width">Fit</button>
    <button id="zoom-in" title="Zoom in">+</button>
    <span id="tail" hidden>rendering rest…</span>
    <span id="error" hidden></span>
  </div>
  <main id="pages"><p id="empty">Waiting for the first compile…</p></main>
  <script src="${stateUri}"></script>
  <script src="${viewerUri}"></script>
</body>
</html>`;
  }
}

module.exports = { PreviewPanel };
