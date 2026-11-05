# TeXyoN Live Preview

A live preview for LaTeX documents that updates as you type, without saving
and without a separate PDF viewer. A resident TeXyoN engine recompiles only
what your edit touched and redraws only the pages that changed, so pages
refresh in a fraction of a normal compile.

## Getting started

1. Open a saved `.tex` file.
2. Run **TeXyoN: Start Live Preview** from the command palette.

The first start downloads the TeX support files it needs and builds the LaTeX
format, which takes a minute or so. Later starts reuse that cache.

The extension ships with its own TeXyoN engine for Linux (x64), macOS (Apple
silicon) and Windows (x64). No TeX distribution is required.

## Features

- **Live preview** of the active document and of included child files, using
  their unsaved buffer contents.
- **Errors and warnings** in the Problems panel and as squiggles, tied to the
  source line TeX names.
- **Forward search**: **TeXyoN: Show Cursor in Live Preview** scrolls to and
  highlights the box for the current source line.
- **Inverse search**: Ctrl-click (Cmd-click on macOS) a spot in the preview
  to jump to its source line. A plain click follows a link.
- **TeXyoN: Export PDF** writes the document next to its main file.
- **TeXyoN: Cancel In-Flight Preview** abandons a long compile, and
  **TeXyoN: Stop Live Preview** shuts the engine down. An engine that stops
  unexpectedly is restarted once.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `texyon.executable` | empty | A TeXyoN binary to use instead of the bundled one. |
| `texyon.format` | `latex` | The XeTeX format to preview with, built on first use. |

## Building from source

To use your own engine build, compile it with the `preview-mupdf` feature
and point `texyon.executable` at it. On Linux and macOS:

```sh
tools/mupdf/build.sh
TEXYON_MUPDF_DIR="$PWD/tools/mupdf/vendor/mupdf" \
  cargo build --release -p texyon --features preview-mupdf
```

On Windows MSVC, install the repository's vcpkg dependencies and Rust archive
tools first:

```powershell
$env:VCPKG_OVERLAY_PORTS = "$PWD\dist\vcpkg-ports"
cargo vcpkg build
rustup component add llvm-tools-preview
$env:VCPKG_ROOT = "$PWD\target\vcpkg"
$env:VCPKGRS_TRIPLET = "x64-windows-static-release"
$env:TEXYON_DEP_BACKEND = "vcpkg"
$env:RUSTFLAGS = "-Ctarget-feature=+crt-static"
cargo build --release -p texyon --features "preview-mupdf,external-harfbuzz"
```

Run the extension's tests with `npm install && npm test`. Set
`TEXYON_PREVIEW_BIN` to a built binary to include the real-server smoke test.

The extension talks to the engine through the session protocol described in
`docs/live-preview/protocol.md`, the same one the web editor uses.

## License

The extension and the bundled engine (which links MuPDF) are distributed under
the GNU AGPL v3 or later. The portable TeXyoN libraries are MIT licensed and
do not enable the MuPDF backend by default.
