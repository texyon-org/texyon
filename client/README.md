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

**`Workspace` owns the text.** One object is the authority on what every file
says; the editor, the compiler and the thing that saves all read from it and are
told when it moves. That is the seam collaborative editing would replace — today
it is a map of strings with one writer, and its internals could become a CRDT
with several without anything outside it changing, because what leaves it is
already what the compiler wants: every file's complete text, at a revision.

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
