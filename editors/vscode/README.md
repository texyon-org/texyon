# TeXyon Live Preview

A live preview for LaTeX documents that updates as you type, without saving
and without a separate PDF viewer. A resident TeXyon engine recompiles only
what your edit touched, so pages refresh in a fraction of a normal compile.

## Getting started

1. Open a saved `.tex` file.
2. Run **TeXyon: Start Live Preview** from the command palette.

The first start downloads the TeX support files it needs and builds the LaTeX
format, which takes a minute or so. Later starts reuse that cache.

The extension ships with its own TeXyon engine for Linux (x64), macOS (Apple
silicon) and Windows (x64). No TeX distribution is required.

## Features

- **Live preview** of the active document and of included child files, using
  their unsaved buffer contents.
- **Forward search**: **TeXyon: Show Cursor in Live Preview** scrolls to and
  highlights the box for the current source line.
- **Inverse search**: Ctrl-click (Cmd-click on macOS) a spot in the preview
  to jump to its source line.
- **TeXyon: Cancel In-Flight Preview** abandons a long compile, and
  **TeXyon: Stop Live Preview** shuts the engine down.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `texyon.executable` | empty | A TeXyon binary to use instead of the bundled one. |
| `texyon.format` | `latex` | The XeTeX format to preview with. |

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
cargo vcpkg build
rustup component add llvm-tools-preview
$env:VCPKG_ROOT = "$PWD\target\vcpkg"
$env:VCPKGRS_TRIPLET = "x64-windows-static-release"
$env:TEXYON_DEP_BACKEND = "vcpkg"
$env:RUSTFLAGS = "-Ctarget-feature=+crt-static"
cargo build --release -p texyon --features "preview-mupdf,external-harfbuzz"
```

Run the extension's tests with `npm test` (no install step is needed). Set
`TEXYON_PREVIEW_BIN` to a built binary to include the real-server smoke test.

## License

The extension and the bundled engine (which links MuPDF) are distributed under
the GNU AGPL v3 or later. The portable TeXyon libraries are MIT licensed and
do not enable the MuPDF backend by default.
