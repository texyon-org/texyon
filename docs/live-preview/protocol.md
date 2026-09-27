# Live-preview JSONL protocol

Protocol version 1 uses one UTF-8 JSON object per physical line. Standard output
is protocol-only; diagnostics use standard error. The server enforces encoded
line, decoded source-file, combined snapshot, and encoded frame limits. Unknown
request variants or fields are rejected.

All request, source, frame, and query counters are canonical decimal strings,
not JSON numbers. Page indices are zero-based; source lines are one-based;
preview coordinates and page sizes are PostScript big points (`bp`).

## Session order

1. Server emits `ready` with `protocol_version`, stable `capabilities`, and the
   frozen support-bundle digest.
2. The first client request must be one `initialize` containing the complete
   primary buffer and any dirty editor-owned children.
3. The server emits increasing `update` frames and accepts the requests below.
4. The client ends with `shutdown`; EOF has the same effect.

```json
{"type":"ready","protocol_version":1,"capabilities":["svg","synctex"],"bundle_digest":"…"}
{"type":"initialize","protocol_version":1,"primary_path":"/work/main.tex","primary_text":"Hello\\bye\n","children":[]}
```

## Client requests

- `replace`: complete UTF-8 `text`; optional normalized `path` selects a child.
- `viewport`: inclusive visible `first` and `last` page.
- `resync`: replay every retained page without compiling.
- `cancel`: durable stop until the next accepted replacement.
- `painted`: `frame_id` plus finite non-negative `paint_ms` telemetry.
- `forward_search`: `query_id`, source `path`, and one-based `line`.
- `backward_search`: `query_id`, `page_index`, `h_bp`, and `v_bp`.
- `shutdown`: clean worker/process termination.

```json
{"type":"replace","text":"Edited\\bye\n"}
{"type":"viewport","first":0,"last":0}
{"type":"forward_search","query_id":"1","path":"main.tex","line":12}
```

## Server events

An `update` contains `frame_id`, `source_revision`, `applied_request_seq`,
`phase`, `outcome`, the complete post-patch `page_count` and `page_sizes`, exact
sorted `changed`/`removed` sets, SVG payloads corresponding exactly to
`changed`, complete per-page unsupported-operation telemetry, operation
metrics, and `tail_pending`. `foreground`, `tail_settle`,
and `resync` frames all receive distinct increasing frame IDs, even at the same
source revision and even when no SVG changed.

Metrics include engine/analysis/render durations and encoded frame bytes, plus
the exact replay rung and decimal-string command count, checkpoint
admission/drop/rejection activity, retained checkpoint rungs/bytes, fresh
shipouts, prefix reuse, pages fingerprinted/rendered, and pages with unsupported
operations.

`cancelled`, `stopped`, `aborted`, and `rejected` do not replace the last-good
viewer belief. Navigation replies echo their query ID and carry either a result
or `null`; a missing source match is normal. `fatal` ends the session.

Both directions follow TeX Live's `synctex` tool: backward search reports the
line `synctex edit` would, and forward search reports the boxes `synctex view`
would, best first. With the `synctex_boxes` capability a forward reply also
carries all of them in `boxes` (omitted when empty) for highlighting, and
`result` is the first.

```json
{"type":"forward_search_result","query_id":"1","result":{"page_index":0,"h_bp":72.0,"v_bp":84.0,"width_bp":31.0,"height_bp":7.0,"depth_bp":2.0},"boxes":[{"page_index":0,"h_bp":72.0,"v_bp":84.0,"width_bp":31.0,"height_bp":7.0,"depth_bp":2.0}]}
{"type":"backward_search_result","query_id":"2","result":{"path":"main.tex","line":12}}
```

Version 1 is strict: incompatible versions and schema additions require an
explicit capability/version design rather than silent interpretation. Clients
must ignore no unknown server event; an unknown event indicates incompatibility
and should terminate the session while retaining the last rendered frame.
