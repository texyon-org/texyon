# TeXyoN

TeXyoN is a LaTeX engine built for live preview. It is XeTeX, ported to Rust
from [Tectonic](https://github.com/tectonic-typesetting/tectonic), kept
resident between edits: an edit replays from a checkpoint just before it,
typesets only up to the pages on screen, and redraws only the pages whose
appearance changed. A keystroke in a long document costs milliseconds rather
than a full compile.

## Where it runs

- **VS Code** ([`packages/vscode`](packages/vscode)): install *TeXyoN Live
  Preview* from the Marketplace or Open VSX. It ships a native engine for
  Linux (x64), macOS (Apple silicon) and Windows (x64), and needs no TeX
  distribution.
- **Web editor** ([texyon-org/editor](https://github.com/texyon-org/editor)):
  the engine compiled to WebAssembly, running in a Web Worker.

Both drive the same session in `crates/live_preview` through the protocol in
`crates/preview_protocol`, so a preview feature lands in both.

## Status

- The XeTeX engine is fully Rust. It is checked against TeX Live's XeTeX on
  about 9,900 documents: the LaTeX kernel's regression tests, the package
  documentation shipped with TeX Live, and a sample of arXiv papers. In the
  last full run, against TeX Live 2026, 9,850 of 9,882 came out identical.
  TeX Live's XeTeX crashes on 3 of the rest, and the others are explained in
  [`known.tsv`](crates/engine_xetex/tests/upstream/known.tsv), mostly bugs in
  XeTeX itself, listed in
  [`UPSTREAM_BUGS.md`](crates/engine_xetex/UPSTREAM_BUGS.md). The 34 documents
  in [`slow.tsv`](crates/engine_xetex/tests/upstream/slow.tsv) are left out,
  because TeX Live's XeTeX cannot finish them or both engines loop.
- PDF output is dvipdfm-x from TeX Live 2026.
- The live preview supports viewport-bounded compiles, pixel or SVG pages,
  errors by source line, source navigation in both directions, and PDF export.
- `texyon compile` runs BibTeX. The live preview does not yet.

## Building

Clone with submodules, since HarfBuzz is built from source:

```sh
git clone --recurse-submodules https://github.com/texyon-org/texyon
```

The engine links fontconfig, FreeType, Graphite2, ICU, libpng, OpenSSL and
zlib. On Debian or Ubuntu:

```sh
sudo apt install build-essential pkg-config libfontconfig1-dev libfreetype-dev \
  libgraphite2-dev libicu-dev libpng-dev libssl-dev zlib1g-dev
```

On macOS, `brew install pkg-config freetype graphite2 icu4c libpng openssl`,
with Homebrew's `icu4c` on `PKG_CONFIG_PATH`. Windows builds with MSVC and
vcpkg, see [`packages/vscode/README.md`](packages/vscode/README.md).

Then:

```sh
cargo build --release -p texyon
target/release/texyon compile document.tex
```

The first compile downloads the TeX Live support files it needs and builds the
format. Later runs reuse that cache.

The live preview needs the MuPDF renderer, which is opt-in because MuPDF is
AGPL licensed. This builds MuPDF and then a binary with the `preview` command:

```sh
cargo xtask mupdf
cargo xtask native --features preview-mupdf
```

Add `--pgo` for the profile-guided release binary. The WebAssembly build is
described in [`crates/wasm`](crates/wasm/README.md). `cargo xtask --help` lists
the build, test and benchmark tasks.

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
| `crates/xtask` | Build, test, benchmark and profiling tasks. |
| `packages/vscode` | The VS Code extension. |
| `packages/wasm` | The `@texyon/wasm` package the web editor uses. |

## Testing

`cargo test --workspace` runs the unit and integration tests, including the
TRIP test. `cargo xtask differential native` checks that incremental
typesetting matches a fresh compile after every keystroke. Changes to engine
behaviour are checked against TeX Live's XeTeX with the upstream harness
(`crates/engine_xetex/tests/upstream/run.py`).

## License

TeXyoN's code is MIT licensed, see [LICENSE](LICENSE). It builds on Tectonic,
XeTeX and TeX, and includes dvipdfmx (GPL v2 or later) for PDF output. Builds
that link MuPDF for the live preview, which includes the VS Code extension and
the web editor, are distributed under the AGPL v3 or later. Details and credits
are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
