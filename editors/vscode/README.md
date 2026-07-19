# Tectonic Live Preview

This development extension opens an extension-owned, SVG-only preview backed by
one resident `tectonic -X preview` process. The executable must be built with
the opt-in `preview-mupdf` feature, and its support bundle and selected format
must already be cached. Preview sessions never fetch from the network.

From the repository root, build it with:

```sh
cargo build -p tectonic --features preview-mupdf
```

Configure `tectonicPreview.executable` if `tectonic` is not on `PATH`, open a
saved TeX document, and run **Tectonic: Start Live Preview**. The extension
sends the primary buffer and every already-dirty eligible child buffer during
initialization, then streams complete UTF-8 buffer replacements.

Run **Tectonic: Show Cursor in Live Preview** to scroll to and highlight the
box associated with the active source line. Ctrl-click on Linux/Windows or
Cmd-click on macOS inside a preview page to reveal the nearest primary or child
source line. Both directions query the last coherent frame and do not trigger a
compile or create a `.synctex.gz` file.

The preview webview accepts only sanitized SVG page payloads. It retains page
slots and scroll position, renders only the visible page plus a one-page
prefetch band, and asks the server for a full resynchronization after a webview
reload.

The native MuPDF preview backend and this extension are distributed under the
GNU AGPL v3 or later. The portable Tectonic libraries do not enable that backend
by default.

Run the reducer and path-authority tests with `npm test` (no install step is
needed).
