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

The committed fixture and dependency-free runner implement this sequence:

```sh
node tools/live-preview/benchmark.mjs \
  --bin target/release/tectonic \
  --output /tmp/tectonic-preview-benchmark
```

The runner uses a multi-page reporting document with editor-owned primary and
child sources. Its early marker is body text immediately after
`\begin{document}`—preamble edits are intentionally a separate worst case. For
each retained sample it:

1. start a new preview process and initialize the unchanged document;
2. discard the initial compile measurement;
3. apply the same early primary edit and report viewport page 0 only;
4. wait for its tail-settle frame;
5. apply the same late child edit and report its visible page;
6. make a same-size replacement to that child marker again to measure the newly
   planted local fence without changing file-size constraints;
7. revert both buffers and confirm byte-for-byte XDV parity with deterministic
   cold batch output;
8. run a ten-replacement burst at 120 ms spacing with `tail_idle_ms=200` and
   confirm that no tail starts between replacements.

Run at least five discarded warmups and thirty retained samples per edit. Store
raw JSONL events and stderr separately. Derive engine and render durations from
frame metrics and end-to-end/paint durations at the client. Report median and
p95 using nearest-rank percentiles; never report only the best sample.

Those warmup/sample counts are the runner defaults. It writes raw `events.jsonl`,
server stderr, request/observation traces, environment metadata, and a
nearest-rank `summary.json` under the selected output directory. The headless
runner measures request-to-frame wall time, foreground-to-tail time, the final
post-fence typing-burst response, and observed cancellation delay; paint time
remains a viewer metric and must be collected from the VS Code client for a
user-facing benchmark. The extension records both request-to-frame and viewer
paint durations in the `Tectonic Live Preview` output channel.

Also retain maximum checkpoint-accounted bytes, rung count, fresh foreground
shipouts, changed/rendered SVG counts, cancellation delay, and tail duration.
Performance is accepted only while all correctness, security, and byte-parity
tests remain green.

## Hardening commands

Run the ordinary output corpus serialized because it mutates one fixture mtime:

```sh
cargo test -p tectonic --test tex-outputs -- --test-threads=1
```

Run the resident differential matrix under AddressSanitizer with an explicit
target so Cargo does not instrument host build scripts. Restricted/ptraced
runners may need leak detection disabled; ASan remains active:

```sh
CARGO_TARGET_DIR=/tmp/tectonic-preview-asan \
ASAN_OPTIONS=detect_leaks=0 \
RUSTFLAGS='-Zsanitizer=address' \
RUSTDOCFLAGS='-Zsanitizer=address' \
cargo +nightly test --target x86_64-unknown-linux-gnu \
  -p tectonic --test live_preview_session -- --test-threads=1
```

Portable protocol/renderer checks, including accidental activation of the
Linux-only adapter feature, are:

```sh
cargo check -p tectonic_preview_protocol --target x86_64-pc-windows-gnu
cargo check -p tectonic_preview_protocol --target wasm32-unknown-unknown
cargo check -p tectonic_preview_render --features backend-mupdf \
  --target x86_64-pc-windows-gnu
cargo check -p tectonic_preview_render --features backend-mupdf \
  --target wasm32-unknown-unknown
```
