# TeXyon Live Preview

This development extension opens an extension-owned, SVG-only preview backed by
one resident `texyon -X preview` process. The executable must be built with
the opt-in `preview-mupdf` feature. The selected format must already be cached;
support files and renderer assets are fetched and cached on demand, as in a
regular compile.

From the repository root, build it with:

```sh
cargo build -p texyon --features preview-mupdf
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

Configure `texyon.executable` if `texyon` is not on `PATH`, open a
saved TeX document, and run **TeXyon: Start Live Preview**. The extension
sends the primary buffer and every already-dirty eligible child buffer during
initialization, then streams complete UTF-8 buffer replacements.

Run **TeXyon: Show Cursor in Live Preview** to scroll to and highlight the
box associated with the active source line. Ctrl-click on Linux/Windows or
Cmd-click on macOS inside a preview page to reveal the nearest primary or child
source line. Both directions query the last coherent frame and do not trigger a
compile or create a `.synctex.gz` file.

The preview webview accepts only sanitized SVG page payloads. It retains page
slots and scroll position, renders only the visible page plus a one-page
prefetch band, and asks the server for a full resynchronization after a webview
reload.

The native MuPDF preview backend and this extension are distributed under the
GNU AGPL v3 or later. The portable TeXyon libraries do not enable that backend
by default.

Run the reducer and path-authority tests with `npm test` (no install step is
needed).
