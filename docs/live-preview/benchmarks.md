# Live-preview benchmarks and checks

Both run real sessions through the protocol, against the native binary
(`texyon -X preview`, built with `preview-mupdf`) or the wasm module under Node.
Documents are project directories around a `main.tex`, or single `.tex` files.
Without any, both use the fixtures in `crates/xtask/corpus`.

## Timing

```sh
cargo xtask bench native
cargo xtask bench wasm --assets target/wasm-assets --rounds 5 path/to/paper
```

Each round replays the editing session in `crates/xtask/src/scenario.rs`: a
cold compile of the first two pages with the rest typeset as the tail, a full
pass, twelve keystrokes typed into prose lines and compiled around their page,
then an export. The table shows the fastest round of each step, and keystrokes
as the median within a round. Compare builds on one machine with the same
documents and bundle.

## Incremental against fresh

```sh
cargo xtask differential native path/to/paper
```

After every keystroke the resident session's pages are drawn and compared with
a fresh session that typeset the edited text from scratch. Undoing every
keystroke must bring back the first pages. A document fails on the first page
that differs, and the command fails if any document did.

## Engine checks

Run the ordinary output corpus serialized because it mutates one fixture mtime:

```sh
cargo test -p texyon --test tex-outputs -- --test-threads=1
```

Run the resident differential matrix under AddressSanitizer with an explicit
target so Cargo does not instrument host build scripts. Restricted/ptraced
runners may need leak detection disabled; ASan remains active:

```sh
CARGO_TARGET_DIR=/tmp/texyon-preview-asan \
ASAN_OPTIONS=detect_leaks=0 \
RUSTFLAGS='-Zsanitizer=address' \
RUSTDOCFLAGS='-Zsanitizer=address' \
cargo +nightly test --target x86_64-unknown-linux-gnu \
  -p texyon --test live_preview_session -- --test-threads=1
```

Portable protocol/renderer checks, including accidental activation of the
Linux-only adapter feature, are:

```sh
cargo check -p texyon_preview_protocol --target x86_64-pc-windows-gnu
cargo check -p texyon_preview_protocol --target wasm32-unknown-unknown
cargo check -p texyon_preview_render --features backend-mupdf \
  --target x86_64-pc-windows-gnu
cargo check -p texyon_preview_render --features backend-mupdf \
  --target wasm32-unknown-unknown
```
