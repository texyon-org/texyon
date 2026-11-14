# @texyon/wasm

TeXyoN, a LaTeX engine built for live preview, compiled to WebAssembly.

```ts
import { TexyonEngine } from "@texyon/wasm";

const engine = await TexyonEngine.create();

const result = await engine.compile({
    "main.tex": source,
    "chapters/one.tex": chapter,
});
for (const page of result.pages) slots[page.index].innerHTML = page.svg;

const { pdf } = await engine.exportPdf();
```

The engine is resident. It keeps one document open, rewinds to the last
checkpoint before an edit instead of starting over, and returns only the pages
whose appearance changed. Everything runs in a Worker.

## Setup

```sh
npm install @texyon/wasm
```

The package ships the module (`texyon_wasm.wasm`) and the support pack
(`texyon-pack.tar.gz`, the format and a common set of classes, packages and
fonts). `TexyonEngine.create()` loads both from next to its own code, and
bundlers copy them into the build. Pass `moduleUrl` and `packUrl` to serve them
from elsewhere. Both are resolved against the page.

`bundleUrl` makes any TeX Live file available. It expects a `.ttb` bundle (read with HTTP
range requests, you can use `https://bundles.texyon.org/texlive2026.ttb`)

The module and pack are kept in Cache Storage and revalidated with the server's
`ETag` or `Last-Modified`, so serve them with one. Files read from a bundle are
cached by bundle URL. Without usable storage the engine starts cold.

To build from source, run `cargo xtask pack` and `cargo xtask wasm`, see
[`crates/wasm`](../../crates/wasm/README.md).
`npm pack` copies their output into `dist`.

## API

### `TexyonEngine.create(options?)`

Starts the engine and resolves once it can typeset. Options: `moduleUrl`,
`packUrl`, `bundleUrl`, `workerUrl`, `primary` (default `main.tex`), `raster`,
`onProgress`, `onLog`, `onRecompileNeeded`.

With `raster`, the Worker draws pages to `ImageBitmap`s instead of returning
SVG, which is much cheaper for the browser to show.

### `engine.compile(files, viewport?, scale?)`

Typesets the project. `files` maps each path to its complete current content,
text or bytes, and a bare string stands for the primary alone. Send everything
every time: unchanged files cost nothing.

`viewport` is an inclusive zero-based page range that bounds the work, leaving
the rest to `continueTail()`. `scale` is pixels per big point for a raster
engine.

The result is a patch: `pages` (SVG) or `rasters` for the changed pages,
`changed` and `removed` indices, and `pageSizes` and `pageOverlays` (links and
destinations) for every page. It also carries `log`, `diagnostics`, `outcome`
and `stats`.

### `engine.continueTail()`

Typesets the next chunk of a viewport-bounded compile. Returns `null` when
nothing is pending.

### `engine.raster(pages, scale)`

Draws published pages of a raster engine at a new scale.

### `engine.forwardSearch(path, line)`, `engine.backwardSearch(page, hBp, vBp)`

SyncTeX in both directions, matching TeX Live's `synctex view` and
`synctex edit`. Coordinates are big points from the page's top-left corner.

### `engine.commands()`

Every command the document currently defines, for completion.

### `engine.exportPdf(onStep?)`

Typesets the whole document until its auxiliary files settle, then writes a PDF
with xdvipdfmx. The frames of those passes come back in `tail`, to apply like
compile results.

### `engine.cancel()`, `engine.close()`

Stop the running compile, keeping the previous preview, or shut the engine down.

## Limits

- One project per engine, since XeTeX keeps process-global state.
- No shell escape.
- Fonts are found by file name, not system name.
- Files come from the project, the pack or the bundle.

## License

The TypeScript in this package is MIT. The bundled engine, `texyon_wasm.wasm`,
links MuPDF and is distributed under AGPL-3.0-or-later as a whole, so serving
it to browsers means offering its source, which is
[texyon-org/texyon](https://github.com/texyon-org/texyon) at the published
version. See `THIRD_PARTY_NOTICES.md`.
