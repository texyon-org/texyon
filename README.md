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
- **VS Code** ([`packages/vscode`](packages/vscode)): a native engine process
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
cargo xtask mupdf
cargo xtask native --features preview-mupdf
```

`cargo xtask native --pgo --features preview-mupdf` builds the profile-guided
release binary. Windows builds with MSVC and vcpkg, see
[`packages/vscode/README.md`](packages/vscode/README.md). The WebAssembly build
is described in [`crates/wasm`](crates/wasm/README.md). `cargo xtask --help`
lists the build, test and benchmark tasks.

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
| `packages/vscode` | The VS Code extension. |
| `packages/wasm` | The `@texyon/wasm` package the web editor uses. |
| `docs/live-preview` | Preview architecture and protocol notes. |

## Testing

`cargo test --workspace` runs the unit and integration tests. Changes to engine
behaviour are checked against TeX Live XeTeX with the upstream harness
(`crates/engine_xetex/tests/upstream/run.py`).

## License

TeXyoN's code is MIT licensed, see [LICENSE](LICENSE). It builds on Tectonic,
XeTeX and TeX, and includes dvipdfmx (GPL v2 or later) for PDF output. Builds
that link MuPDF for the live preview, which includes the VS Code extension and
the web editor, are distributed under the AGPL v3 or later. Details and credits
are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
