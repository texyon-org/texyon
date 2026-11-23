# TeXyoN

TeXyoN (after tachyon, a hypothetical faster-than-light particle) is a TeX engine based on XeTeX, written in Rust, and designed for fast preview between edits. A keystroke in a long document costs milliseconds rather than a full compile.

TeXyoN is forked from [Tectonic](https://github.com/tectonic-typesetting/tectonic), because of its self-contained design.

> [!WARNING]
> TeXyoN is still in early development. Full compilation output is validated on over 9,000 documents, but expect large code changes and new features. Read more about its design here: https://yendric.be/introducing-texyon/

## What you get

- **VS Code** ([`packages/vscode`](packages/vscode)): install _TeXyoN Live Preview_ from the marketplace. It ships a native engine for Linux (x64), macOS (Apple silicon) and Windows (x64), and needs no TeX distribution.
- **npm** ([`packages/wasm`](packages/wasm)): `npm install @texyon/wasm` for the engine compiled to WebAssembly, running in a Web Worker behind a small API, to build your own editor or preview in the browser.

Both drive the same session in `crates/live_preview` through the protocol in `crates/preview_protocol`.

## Building

Clone with submodules, since HarfBuzz is built from source:

```sh
git clone --recurse-submodules https://github.com/texyon-org/texyon
```

The engine links fontconfig, FreeType, Graphite2, ICU, libpng, OpenSSL and zlib. On Debian or Ubuntu:

```sh
sudo apt install build-essential pkg-config libfontconfig1-dev libfreetype-dev \
  libgraphite2-dev libicu-dev libpng-dev libssl-dev zlib1g-dev
```

On macOS, `brew install pkg-config freetype graphite2 icu4c libpng openssl`, with Homebrew's `icu4c` on `PKG_CONFIG_PATH`. Windows builds with MSVC and vcpkg, see [`packages/vscode/README.md`](packages/vscode/README.md).

Then:

```sh
cargo build --release -p texyon
target/release/texyon compile document.tex
```

The initial compile downloads the TeX Live support files it needs and builds the format, so the first compile takes longer until these are cached.

The live preview needs the MuPDF renderer, which is opt-in because MuPDF is AGPL licensed. This builds MuPDF and then a binary with the `preview` command:

```sh
cargo xtask mupdf
cargo xtask native --features preview-mupdf
```

Add `--pgo` for the profile-guided release binary. The WebAssembly build is described in [`crates/wasm`](crates/wasm/README.md). `cargo xtask --help` lists the build, test and benchmark tasks.

## Testing

`cargo test --workspace` runs the unit and integration tests, including the TRIP test. `cargo xtask differential native` checks that incremental typesetting matches a fresh compile after every keystroke. Changes to engine behaviour are checked against TeX Live's XeTeX with the upstream harness (`crates/engine_xetex/tests/upstream/run.py`).

## Thanks

TeXyoN stands on decades of work by others. Thanks to Donald Knuth for TeX, Jonathan Kew and the XeTeX maintainers for Unicode and OpenType in TeX, the TeX Live team for keeping it all building and packaged every year, and the Tectonic team, whose self-contained engine this started from. Thanks also to the authors of dvipdfmx, the LaTeX Project and every package author on CTAN, the HarfBuzz, ICU, Graphite and MuPDF projects, and TeXpresso for showing that a resident TeX engine could work.

## License

TeXyoN's code is MIT licensed, see [LICENSE](LICENSE). It builds on Tectonic, XeTeX and TeX, and includes dvipdfmx (GPL v2 or later) for PDF output. Builds that link MuPDF for the live preview, which includes the VS Code extension and `@texyon/wasm`, are distributed under the AGPL v3 or later. Details and credits are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
