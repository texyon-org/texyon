# Live-preview benchmark recipe

Benchmark numbers are meaningful only with the exact fixture, source edits,
toolchain, feature set, bundle/format digest, and machine metadata recorded
beside the samples.

## Build and environment

```sh
cargo build --release --bin tectonic --features preview-mupdf
rustc -Vv
uname -a
```

Use a fully cached bundle and format. Record the `ready.bundle_digest`, selected
format, CPU model, total RAM, operating system, MuPDF archive identity, Git
commit, and whether rewind verification is enabled. Run no network-capable
provider after bootstrap.

## Reference sequence

Use one committed multi-page reporting document with editor-owned primary and
child sources. For each retained sample:

1. start a new preview process and initialize the unchanged document;
2. discard the initial compile measurement;
3. apply the same early primary edit and report viewport page 0 only;
4. wait for its tail-settle frame;
5. apply the same late child edit and report its visible page;
6. revert both buffers and confirm byte-for-byte XDV parity with deterministic
   cold batch output;
7. run a ten-replacement burst at 120 ms spacing with `tail_idle_ms=200` and
   confirm that no tail starts between replacements.

Run at least five discarded warmups and thirty retained samples per edit. Store
raw JSONL events and stderr separately. Derive engine and render durations from
frame metrics and end-to-end/paint durations at the client. Report median and
p95 using nearest-rank percentiles; never report only the best sample.

Also retain maximum checkpoint-accounted bytes, rung count, fresh foreground
shipouts, changed/rendered SVG counts, cancellation delay, and tail duration.
Performance is accepted only while all correctness, security, and byte-parity
tests remain green.
