# Live-preview architecture

Tectonic live preview is one coherent pipeline, not a second TeX engine. The
normal deterministic batch compiler remains the output oracle. Preview adds
resident ownership, replay fences, selective rendering, and a bounded local
transport around the same XeTeX implementation.

## Modules

1. **Source authority** (`src/live_preview/source.rs`) owns one canonical
   document root, immutable UTF-8 primary/child buffers, accepted revisions,
   and whole-buffer replacement validation.
2. **Worker and request clock** (`worker.rs`, `worker_clock.rs`) serialize all
   engine access, keep request/source/frame counters separate, debounce tail
   work, and cooperatively supersede stale work.
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
8. **Publication and transport** (`pipeline.rs`, `publication.rs`,
   `transport.rs`, `server.rs`) atomically commit the assembled document,
   source index, page ledger, and JSONL frame only after every selected SVG is
   available.
9. **VS Code client** (`editors/vscode`) keeps page slots and scroll position,
   installs changed off-screen SVG lazily, reports the true viewport, and
   performs forward/inverse source navigation against the last frame.

## Invariants

- A worker has exactly one engine state and one accepted source snapshot in
  flight. TeX work never runs concurrently inside a process.
- `RequestSeq`, `SourceRevision`, and `FrameId` are independent checked `u64`
  counters encoded as decimal strings.
- A rejected request advances applied request accounting but neither source
  revision nor viewer belief.
- Cancellation, abort, renderer failure, or output failure cannot partially
  publish engine, page, SVG, or source-navigation state.
- A replay fence constrains every editor or root-confined disk source through
  the furthest byte prefix, minimum/exact size, and EOF facts actually
  observed. Unread or prefix-only probes may tolerate later changes; consumed
  bytes and observed metadata must remain identical.
- A viewport foreground compiles through exactly the requested last page. Old
  tail visuals and SyncTeX records remain the client belief until one idle tail
  continuation atomically settles them.
- Queries read only the last successfully published belief and never cause a
  compile.
- After `ready`, support resources are cache-only, source disk reads are
  confined to the canonical document root, shell escape is disabled, and all
  operation outputs are memory-only.

## Failure model

Normal TeX errors and cooperative cancellation rewind to a valid rung and keep
the last coherent preview. Invalid source requests are rejected. A poisoned
engine, panic, protocol violation, channel failure, or atomic sink failure is
fatal to the session; the editor must start a new process. Shutdown repeatedly
requests cooperative cancellation while joining, but it is not an in-process
hard kill for an uninstrumented blocking native call.
