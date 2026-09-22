# The Tectonic editor

A LaTeX editor that compiles in your browser. There is no build queue, because
there is no build server: the engine is a resident WebAssembly session in a
Worker, and an edit replays from a checkpoint just before your cursor rather
than starting over.

```
npm install
npm run dev      # needs ../js built, and the engine assets (below)
```

## The engine's assets

The client serves the compiled engine and its support pack from `public/`:

```
tools/wasm/build.sh          # public/tectonic/tectonic_wasm.wasm
tools/wasm/build-assets.sh   # public/tectonic/manifest.json and files/
```

Neither is in the repository — together they are about sixty megabytes, and
both are build outputs. The browser caches them across loads, so paying for
them is a first-visit cost rather than a per-reload one.

## How it is put together

**A shared document owns a project's text, and `Workspace` mirrors it.** Each
project is one Yjs document (`src/collab/`), edited live by everyone it is shared
with. `CollabBinding` keeps a `Workspace` in step with it, so the compiler,
completion and the file tree read plain files at a revision and never see Yjs.
Local and remote edits reach the engine the same way: the editor changes the
document, the document changes the workspace. The sample, with no server
behind it, edits the workspace directly.

**The engine is fed the whole project on every change.** Not a concession — the
intended usage. It diffs each file against what it last saw, so untouched files
cost nothing and the one you edited replays only from the point that changed.

**There is no debounce.** A compile in flight is left alone, and the run that
finishes checks whether the workspace moved meanwhile and goes again, so typing
fast coalesces into one follow-up run at whatever rate the engine can keep up
with. A timer on top of that would add its own delay to every edit — and on a
warm document that delay would dwarf the ten milliseconds it was waiting for.

**The preview is patched, not redrawn.** The engine returns only the pages whose
appearance changed, so the viewer keeps a DOM slot per page and replaces the
contents of just those. A one-word edit in a twenty-page document touches one
slot; the other nineteen are never re-parsed, re-laid-out or repainted.

## What is in the panes

- **Files** — the tree is inferred from the slashes in the project's paths,
  because a directory with nothing in it is not something a project can have.
- **Editor** — CodeMirror 6. Completion knows what *this* project declares:
  labels inside `\ref{…}`, citation keys inside `\cite{…}`, and the commands and
  environments its own preamble defines. Diagnostics from the TeX transcript sit
  in the gutter, attributed to the file that actually caused them.
- **Preview** — zoom, clickable `\url` and `\ref`, and SyncTeX both ways:
  Ctrl-J shows where the cursor's line landed, and double-clicking the page
  opens the source that produced it.

## Checks

```
npm test        # the pure parts: the workspace, the symbol index
npm run typecheck
npm run build
```

The interesting failures here are not type errors. The editor is a Worker, a
WebAssembly module and a resident TeX session, and the way to know it works is
to load it: `npm run build && npm run preview`, then type.
