# TeXyoN

TeXyoN is a LaTeX engine built for live preview. It is XeTeX, ported to Rust
from [Tectonic](https://github.com/tectonic-typesetting/tectonic), kept
resident between edits: an edit replays from a checkpoint just before it,
typesets only up to the pages on screen, and redraws only the pages whose
appearance changed. A keystroke in a long document costs milliseconds rather
than a full compile.

## Where it runs

- **Web editor** ([texyon-org/editor](https://github.com/texyon-org/editor)):
  the engine compiled to WebAssembly, running in a Web Worker.
- **VS Code** ([`editors/vscode`](editors/vscode)): a native engine process
  behind a preview panel. Install it from the Marketplace as *TeXyoN Live
  Preview*.

Both speak the same [session protocol](docs/live-preview/protocol.md) to the
same session in `crates/live_preview`, so a preview feature lands in both.

## Status

- The XeTeX engine is fully Rust and matches the reference output on the
  repository's 639-document corpus.
- Live preview supports viewport-bounded compiles, pixel or SVG pages, errors
  by source line, source navigation in both directions, and PDF export.
- The preview does not run BibTeX yet.
- The native preview runs on Linux and Windows. The macOS build does not wire
  up the MuPDF renderer yet, so its extension cannot preview.

## Building

The command-line program has three commands: `compile`, `preview` and
`bundle`.

```sh
cargo build --release -p texyon
target/release/texyon compile document.tex
```

The live preview needs the MuPDF renderer, which is opt-in because MuPDF is
AGPL licensed. On Linux:

```sh
tools/mupdf/build.sh
TEXYON_MUPDF_DIR="$PWD/tools/mupdf/vendor/mupdf" \
  cargo build --release -p texyon --features preview-mupdf
```

Windows builds with MSVC and vcpkg, see
[`editors/vscode/README.md`](editors/vscode/README.md). The WebAssembly build
lives in `crates/wasm` and is assembled by the web editor's
`tools/build-engine.sh`.

The first compile downloads the TeX Live support files it needs and builds the
format, and later runs reuse that cache.

## Layout

| Path | What it is |
| --- | --- |
| `crates/engine_xetex` | The XeTeX engine, with its resident session and checkpoints. See its `ARCHITECTURE.md`. |
| `crates/live_preview` | The preview session every host drives, and the native stream server. |
| `crates/preview_render` | XDV analysis and page rendering (MuPDF). |
| `crates/preview_protocol` | The messages hosts exchange with a session. |
| `crates/wasm` | The WebAssembly host. |
| `crates/engine_xdvipdfmx`, `crates/pdf_io` | PDF output. |
| `crates/engine_bibtex` | BibTeX. |
| `crates/bridge_*`, `crates/xetex_layout` | FreeType, HarfBuzz, ICU, fontconfig and font layout. |
| `crates/bundles`, `crates/geturl` | TeX Live support bundles and their download. |
| `editors/vscode` | The VS Code extension. |
| `js` | The `@texyon/wasm` package the web editor uses. |
| `docs/live-preview` | Preview architecture and protocol notes. |

## Testing

`cargo test --workspace` runs the unit and integration tests. Changes to engine
behaviour are checked against the goldens corpus
(`crates/engine_xetex/scripts/corpus.sh`) and the upstream TeX Live harness
(`crates/engine_xetex/tests/upstream/run.py`).

## License

The libraries are MIT licensed. The MuPDF renderer backend, the native preview
binary that links it, and the VS Code extension are AGPL v3 or later.
