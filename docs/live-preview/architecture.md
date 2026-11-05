# Live-preview architecture

TeXyoN live preview is one coherent pipeline, not a second TeX engine. The
normal deterministic batch compiler remains the output oracle. Preview adds
resident ownership, replay fences, selective rendering, and a bounded local
transport around the same XeTeX implementation.

## Modules

Everything from here to the renderer lives in `crates/live_preview`. It is a
library rather than part of the `tectonic` binary so that the CLI and the
browser build can serve the same pipeline; the CLI is one caller of it, not its
owner.

Its `native` feature, on by default, is the line between the two. Without it the
crate drops the support-bundle resource provider and the stream server, the
things that need a filesystem, threads and stdio, and what remains compiles
for `wasm32-wasip1`: the session, source authority, the engine driver,
assembly, publication and both SyncTeX searches. A document root is `Filesystem`-backed or `Detached`; the first
resolves paths through the real filesystem so that a symlink cannot leave the
root, and the second has no filesystem to resolve through, which is also why it
has no symlinks to guard against.

1. **Source authority** (`source.rs`) owns one canonical document root,
   immutable UTF-8 primary/child buffers, accepted revisions, and whole-buffer
   replacement validation.
2. **Session** (`session.rs`, `dispatch.rs`) is the one interface every host
   drives: it stages edits by revision, compiles, settles tails, draws pages to
   pixels or SVG, parses diagnostics, exports PDF and answers searches. A host
   (`crates/wasm`, `native.rs`) only supplies I/O, tuning and transport.
3. **Resident engine driver** (`engine_driver.rs`, engine `session.rs`) owns the
   non-`Send` XeTeX state on its worker thread and converts each operation into
   XDV, page facts, source geometry, and a recoverable/fatal outcome.
4. **Replay and checkpoints** (engine `checkpoint/`, `replay/`) journal first
   writes, retain byte-accounted rungs, reconstruct input/output handles, and
   select only fences whose exact consumed bytes remain valid.
5. **Page facts and source geometry** (engine `page_manifest.rs`,
   `sync_index.rs`) record XDV spans and in-memory SyncTeX rectangles at
   shipout. Resident sessions never serialize `.synctex.gz`.
6. **Document assembler** (`assembler.rs`) reconstructs absolute XDV tails and
   revalidates the preamble, trailer, postamble pointer, page count, and
   backwards BOP chain before exposing bytes.
7. **Renderer** (`crates/preview_render`) analyzes XDV using immutable resource
   bytes, fingerprints complete page meaning, and renders only selected pages.
   Its core is portable; the native MuPDF SVG adapter exists only on Linux.
8. **Publication** (`pipeline.rs`, `publication.rs`) atomically commits the
   assembled document, source index and page ledger only after every selected
   page is available.
9. **Native server** (`native.rs`) runs the session on its own thread and
   speaks [the session protocol](protocol.md) over stdio. A reader thread
   applies `cancel` at once.
10. **VS Code client** (`editors/vscode`) keeps one compile in flight, draws
    the pixels it gets on canvases, asks for stale pages as they scroll into
    view, and shows diagnostics in the Problems panel.

## Invariants

- A session has exactly one engine state and one accepted source snapshot in
  flight. TeX work never runs concurrently inside a process.
- `RequestSeq`, `SourceRevision`, and `FrameId` are independent checked `u64`
  counters.
- A rejected compile changes neither the accepted sources nor the viewer
  belief, and the client then sends every text whole.
- Cancellation, abort, renderer failure, or output failure cannot partially
  publish engine, page, SVG, or source-navigation state.
- A replay fence constrains every editor or root-confined disk source it opened
  by exactly one number: the furthest byte the engine reached on that channel,
  by reading or by seeking. A source may change at or beyond its watermark,
  because nothing there has been looked at. A channel whose watermark reached
  the end of the file was consumed entirely and the engine acted on the file
  ending there, so any change to it invalidates the fence, appending included.
  A source that disappeared, or that gained or lost all of its bytes, never
  survives.
- A query answered without moving a cursor therefore constrains nothing.
  Resolving `\input` and graphics paths asks for a size and reads no byte; the
  branch taken on the answer is the only thing that mattered, and existence and
  emptiness still hold. A document that instead typesets or numerically
  compares such a size keeps a stale value in the preview until the next full
  compile; the corpus runner's cold-session oracle is what proves a document is
  unaffected.
- A viewport foreground compiles through exactly the requested last page. Old
  tail visuals and SyncTeX records remain the client belief until one idle tail
  continuation atomically settles them.
- Queries read only the last successfully published belief and never cause a
  compile.
- After `open`, support resources use the normal bundle cache and fetch missing
  files on demand. Source disk reads are confined to the canonical document
  root, shell escape is disabled, and all operation outputs are memory-only.

## Failure model

Normal TeX errors and cooperative cancellation rewind to a valid rung and keep
the last coherent preview. Invalid source requests are rejected. A poisoned
engine, panic, protocol violation, channel failure, or atomic sink failure is
fatal to the session. The VS Code client restarts the process once. Shutdown repeatedly
requests cooperative cancellation while joining, but it is not an in-process
hard kill for an uninstrumented blocking native call.
