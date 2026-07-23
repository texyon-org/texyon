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

Run real editable documents through the incremental pipeline with a manifest:

```text
# id<TAB>document root<TAB>root-relative primary
report-001	/path/to/report-001	main.tex
report-002	/path/to/report-002	paper/main.tex
```

```sh
node tools/live-preview/corpus.mjs \
  --bin target/release/tectonic \
  --manifest /path/to/live-preview-corpus.tsv \
  --output /tmp/tectonic-preview-corpus \
  --verify-rewinds
```

The runner loads UTF-8 TeX/LaTeX sources beneath each document root, follows
statically resolvable `\input` and `\include` chains, and edits the deepest
sources first. It replaces safe prose near the source tail with a visible
length-changing marker followed by same-size marker variants (falling back to
non-rendered instrumentation when needed). If a prose mutation produces a TeX
error, the runner first proves that a fresh session produces the same error,
restores and verifies the original document, then tries another bounded
candidate. An incremental-only error remains a hard differential failure. Every
incremental full-viewer resync is compared against a fresh preview-session
oracle; the original bytes are then restored and compared with the initial
belief. The JSON report records input depth, rendered-output changes, and
nonzero replay counts as coverage signals. Crashes, unexpected fatal exits,
aborts, timeouts,
checkpoint-budget violations, and SVG/page-belief differences fail the
document while preserving a trace.

Inputs that cannot cross the current production protocol boundary are reported
separately as `unsupported`, with a stable reason code: non-UTF-8 editor
sources, individual sources above 16 MiB, snapshots above 64 MiB, or encoded
publication frames above 64 MiB.
An incremental frame-limit failure is classified this way only after a fresh
session hits the same limit for the exact edited snapshot; otherwise it is a
hard differential failure. Unsupported entries never hide hard failures in
the report. The runner exits 1 when any
document fails, 2 when there are only unsupported documents, and 0 only when
every manifest entry passes.

The manifest root may be broader than the primary path, but the runner scans
children from the primary file's parent because that is the authority root
used by the production JSONL server.

The committed smoke manifest includes a three-level nested input stack:

```sh
node tools/live-preview/corpus.mjs \
  --bin target/release/tectonic \
  --manifest tools/live-preview/fixtures/corpus.tsv \
  --output /tmp/tectonic-preview-smoke \
  --verify-rewinds
```

Use `--max-documents` for a quick gate on a larger manifest; omit it for the
complete corpus.

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
