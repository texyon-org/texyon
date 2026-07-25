# `@tectonic/wasm`

Tectonic's LaTeX engine, compiled to WebAssembly, with a TypeScript API built
for editors.

```ts
import { TectonicEngine } from "@tectonic/wasm";

const engine = await TectonicEngine.create({
  onProgress: (p) => console.log(p.phase),
});

const result = await engine.compile(source);

// A patch: only the pages that changed.
for (const page of result.pages) {
  slots[page.index].innerHTML = page.svg;
}

// The same typesetting, as a PDF, when someone wants to keep it.
const { pdf } = await engine.exportPdf();
```

## The idea

The engine is **resident**. It keeps one document open and you hand it each new
version of the text; it does not start over.

That distinction is the whole point, and it applies twice.

**The engine does not restart.** A cold LaTeX run spends nearly all of its time
before your cursor — reading a 24 MB format dump, loading the document class,
running the preamble, typesetting everything above the edit. A resident session
pays that once, then rewinds to the deepest checkpoint whose inputs are still
byte-for-byte valid. `result.stats.replayRung` reports how deep that was;
anything above zero means the preamble was not re-run.

**The renderer does not redraw.** It walks the engine's output, fingerprints
what each page *means*, and returns SVG only for the pages whose fingerprint
moved. Editing the last paragraph of a twenty-six page document costs 4 ms of
TeX, 2 ms of analysis and 1 ms of drawing — one page.

Everything runs in a Worker, so the page stays responsive even while TeX is
blocked waiting for a support file.

## Setup

Two build artifacts have to be reachable from the page:

- `tectonic/tectonic_wasm.wasm` — the engine, from `tools/wasm/build.sh`
- `tectonic/manifest.json` and `tectonic/files/` — the support pack, from
  `tools/wasm/build-assets.sh`

Optionally a third, and it is what makes arbitrary `\usepackage` work:

- `bundleUrl` — a Tectonic bundle tar. Its index is fetched once in the
  background; individual files are then single HTTP range requests. The host has
  to allow cross-origin range requests, which `relay.fullyjustified.net` does
  not, so serve it from your own mirror or proxy it.

Both paths are configurable through {@link EngineOptions}. They are resolved
against the page, not against the Worker script.

Your server must answer a missing support file with a **404**, not with an
`index.html` fallback. A single-page-app host that rewrites unknown paths will
otherwise hand the engine a web page and tell it that it is `pdftex.map`. The
Worker checks each file's length against the manifest and reports a clear error
if this happens, but the fix is on the server.

## API

### `TectonicEngine.create(options?)`

Starts the engine. Resolves once the module is instantiated and the format dump
is loaded — support files keep arriving in the background afterwards.

Options: `moduleUrl`, `supportUrl`, `workerUrl`, `primary`, `prefetch`,
`onProgress`, `onLog`.

`prefetch` (on by default) pulls the whole support pack into memory once the
engine is up. Keep it on. Without it the first compile blocks on a few hundred
synchronous requests, taken from inside the typesetting loop.

### `engine.compile(text, viewport?)`

Typesets `text`, which is the document's complete current contents. The engine
diffs it against what it last saw, so sending the whole buffer is both the
intended usage and the cheap one.

`viewport` is an inclusive, zero-based `[first, last]` page range. Passing it
bounds the foreground work to the visible pages and leaves the rest for
`continueTail()` — which is what keeps latency flat as a document grows.

Returns a `CompileResult`, which is a **patch**: `pages` holds SVG for the
changed pages only, `changed` their indices, `removed` the indices the document
no longer has, and `pageSizes` every page's size so a viewer can lay out a slot
before it has the page. Plus `pageCount`, the TeX `log`, an `outcome` of
`spotless | warnings | errors`, and `stats`.

An empty `pages` means nothing about the document's appearance changed — retype
the same characters and that is exactly what you get.

### `engine.continueTail()`

Finishes a viewport-bounded compile through the end of the document. Returns
`null` if nothing was pending. Worth calling when the user pauses.

### `engine.exportPdf()`

Returns the document as a PDF, in `pdf`.

This is not a second compile. It is the preview's own assembled XDV run through
xdvipdfmx, so a download cannot disagree with what the viewer was showing — the
same typesetting, written out differently.

A viewport-bounded compile stops at the last visible page, so this settles the
document first and returns the frames that took in `tail`. Apply them as you
would a compile result, or the viewer's pages fall behind the PDF the user just
got. `tail` is empty when the document was already finished, which is the
common case.

### `engine.cancel()`

Asks the running compile to stop at its next cancellation point. The engine
rewinds to its last good state, so the current preview stays valid.

### `engine.close()`

Shuts the engine down and terminates its Worker.

## Limits

- **One document per engine.** XeTeX keeps process-global state, so a module
  instance hosts a single session. A second document means a second engine, and
  therefore a second Worker and a second copy of the format dump.
- **No shell-escape and no subprocesses**, so `minted` and friends cannot work.
- **Fonts are found by filename, not by system name.** `\setmainfont{Palatino}`
  will not resolve; `\setmainfont{...}[Path=...]` and
  `\font\f="[file.otf]"` will.
- **Files must come from the pack or the bundle.** Set `bundleUrl` and all of
  TeX Live is reachable; without it you are limited to whatever
  `tools/wasm/build-assets.sh` put in the pack. Either way there is no path for
  the user's own local files.
