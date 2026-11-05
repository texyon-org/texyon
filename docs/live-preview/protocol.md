# Live-preview session protocol

Every host speaks the same requests and responses, defined once in
`crates/preview_protocol/src/session.rs` and answered by `live_preview`'s
`handle`. Field names are camelCase. Page indices are zero-based, source lines
are one-based, and positions and page sizes are PostScript big points (`bp`).

Hosts differ only in transport and in where bulk bytes (SVG, pixels, PDF) live:

- **Native** (`texyon -X preview --protocol 2`): each frame on stdin and stdout
  is a little-endian `u32` length, then a MessagePack map. Bulk bytes are an
  inline `data` binary field. Standard error carries logs. The server refuses
  a `--protocol` it does not speak.
- **Wasm** (`texyon_request`): one JSON request in, one JSON response out.
  Bulk bytes stay in linear memory, as `ptr` and `len` fields.

## Requests

Each request is a map with an `op`. The server answers in order, one response
per request, with one exception: `cancel` stops the running operation at once
and gets no reply, so a client sends it only while it waits on the operation
it means to stop.

- `open`: `format`, the `primary` source relative to `root`, the project
  `root` (required natively), and `raster` to draw pages to pixels instead of
  SVG.
- `compile`: `files` maps every source path the client holds to its revision.
  `edits` carries one UTF-16 splice per file from a revision the session holds
  (`base`, `from`, `to`, `insert`), and `texts` carries whole texts. Any other
  revision the session lacks is read from the host (the project directory,
  natively). `viewport` bounds the foreground work and `scale` is the pixels
  per bp to draw the changed pages around it at.
- `continueTail`: typeset what a viewport-bounded compile left. `idle` when
  nothing was pending.
- `raster`: draw `pages` at `scale`.
- `export`: the current document as PDF. Refused while a tail is pending.
- `forwardSearch` (`path`, `line`), `backwardSearch` (`pageIndex`, `hBp`, `vBp`).
- `commands`: the control sequences the engine has defined, for completion.
- `cancel`, `shutdown`.
- `preload` (wasm only): a support file the host already holds.

```json
{"op":"compile","files":{"main.tex":4},"edits":{"main.tex":{"base":3,"from":120,"to":120,"insert":"x"}},"texts":{},"viewport":[2,3],"scale":2.0}
```

## Responses

Each response has a `status`: `ok`, `compiled`, `rastered`, `exported`,
`located`, `commands`, `idle` or `error` with a `message`.

`compiled` is a patch. `published` is false for a cancelled or abandoned run,
whose page fields mean "keep what you have". Otherwise `changed` and `removed`
are exact, `rasters` (or `pages`, for SVG) hold the changed pages around the
viewport, and `pageSizes` and `pageOverlays` (links and destinations) cover
every page. A changed page outside that band is stale until a `raster` request
draws it. `diagnostics` gives errors and warnings by source line, recovered from
the transcript in `log`. `tailPending` says whether pages past the viewport are
left to typeset. The remaining fields are engine and renderer counters.

Pixels are width and height as little-endian `u32`, then RGBA rows.

Both searches follow TeX Live's `synctex` tool: backward search reports the
line `synctex edit` would, and forward search reports as `rect` the best box
`synctex view` would, to scroll to. `boxes` (omitted when empty) are the boxes
to flash, on the page of `rect`. A line that produced nothing falls back to the
nearest line that did, looking below it first, since a beamer frame is typeset
under its `\end{frame}` line.

## Client policy

The VS Code client and the web editor keep one compile in flight. Edits made
meanwhile merge into the next compile, so fast typing still shows a frame per
compile. Only tail work is cancelled by an edit, after a short pause before it
starts, since nobody looks past the viewport while typing.
