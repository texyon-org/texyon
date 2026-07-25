# Tectonic browser demo

A LaTeX editor with a live preview, running entirely in the browser.

```sh
cargo build --release                      # the native binary the pack builder needs
tools/wasm/fetch-deps.sh --with-mupdf      # C libraries, including the renderer
tools/wasm/build.sh                        # the engine
tools/wasm/build-assets.sh                 # the support pack
cd demo && npm install && npm run dev
```

Then open the printed URL. First load fetches about 46 MB — 24 MB of it the
LaTeX format dump — and the browser caches it afterwards.

Everything in TeX Live is available: the pack covers a common core so the first
compile never touches the network, and anything else is pulled from the Tectonic
bundle on demand, one HTTP range request per file. The first document to use a
big package like TikZ pays a few seconds while its hundreds of files arrive;
after that they are resident and edits are back to single-digit milliseconds.

## What to look at

The first number in the toolbar is **keystroke to pixels**: measured from the
keystroke that triggered the compile to the frame in which the new page is on
screen, not from when the engine started.

Next to it are the three costs that make it up — TeX, XDV analysis, drawing —
and then **n/m redrawn**: how many of the document's pages had to be drawn
again. Edit the last paragraph of a long document and it stays at 1. Retype the
same characters and it drops to 0, because nothing about the page changed.
**rung** is how deep into the document the checkpoint it resumed from was;
anything above 0 means the preamble was never re-run.

## How the latency budget is spent

Measured, not estimated. Engine numbers come from `tools/wasm/smoke-test.mjs`;
the end-to-end number is what the app itself reports over a 45-keystroke typing
burst in headless Chrome.

Editing near the end of a 26-page document:

| | |
| --- | --- |
| XeTeX, incremental edit | **4 ms** |
| XDV analysis | **2 ms** |
| Drawing the changed page | **1 ms** |
| pages redrawn | **1 of 26** |
| **keystroke to pixels** (1-page doc, in the browser) | **median 53 ms, p90 60 ms** |

A cold compile of that document is 413 ms, so a warm edit is roughly 60× cheaper.
What is left in the browser number is the 16 ms debounce plus the Worker
round-trip and two animation frames — not typesetting.

Three things get it there.

**The engine session is resident.** An edit resumes from a checkpoint just
before it rather than from the top: about 42,000 TeX commands instead of
1.4 million, a 52× reduction. The preamble, the class file and every page above
the cursor are not re-executed. The `rung` in the toolbar is how deep into the
document that checkpoint was.

**Only pages that changed are redrawn.** The renderer fingerprints what each
page *means*, so an edit in the last paragraph redraws one page out of
twenty-six and reuses the analysis of the other twenty-four. Retyping identical
text redraws nothing at all.

**Pages are DOM slots, patched in place.** The engine returns SVG for the
changed pages only, so the viewer replaces one slot's contents and leaves the
rest untouched — never re-parsed, never re-laid-out, never repainted.

**The font maps are trimmed to the fonts in the pack.** `pdftex.map` ships
38,046 lines, and anything parsing all of it per keystroke pays ~250 ms for
fonts the build does not have. `build-assets.sh` cuts it to the entries whose
files are present.

## Files

| | |
| --- | --- |
| `sample.tex` | The starting document. Also compiled by `build-assets.sh`, so the pack is guaranteed to cover it. |
| `src/usePreview.ts` | Owns the engine and the timing. |
| `src/Preview.tsx` | Installs the changed pages' SVG into per-page DOM slots. |
| `src/Editor.tsx` | CodeMirror, set up once and left alone so typing never contends with React. |

## If you rebuild the pack

Restart the dev server afterwards. Vite caches its view of `public/`, and
`build-assets.sh` deletes and recreates that directory — without a restart Vite
answers support-file requests with `index.html`, and the engine tries to parse a
web page as a font map.
