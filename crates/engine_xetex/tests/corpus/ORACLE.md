# Corpus oracle & determinism pins

This file is the provenance record for the parity corpus (see
`plan/08-parity-corpus.md`). Everything here must be reproducible.

## Oracle (pre-Rust reference engine)

- **Baseline commit:** `1a99fcf5` — "Security: Fix 5 Dependabot alerts
  (#1348)". Last commit before `231b13d2` (plan added) and `5806c164`
  (Phase 0 scaffolding). `engine_xetex` is pure C here; zero Rust.
- **Tag:** `oracle-pre-rust` (create with
  `git tag oracle-pre-rust 1a99fcf5`).
- **Build:**
  ```sh
  git worktree add ../tectonic-oracle oracle-pre-rust
  cargo build --release -p tectonic --manifest-path ../tectonic-oracle/Cargo.toml
  # ORACLE_BIN=../tectonic-oracle/target/release/tectonic
  ```
- **Cherry-picks applied to the oracle worktree:** none yet. If the
  oracle ever needs a non-engine commit (bundle/config plumbing) to
  resolve the pinned bundle, record the cherry-picked SHAs here. Never
  cherry-pick an `engine_xetex` commit.

## Bundle pin

- **Prefix:** `https://relay.fullyjustified.net` (default,
  `crates/bundles/src/lib.rs:39`).
- **Resolved bundle:** `https://data1.fullyjustified.net/tlextras-2022.0r0.tar`
- **Content index SHA-256:**
  `6ffe055852f8faf66c0acbe1a7fb27f87b869a90bad1204f3bf4d9683f597c7c`
- The bundle is fetch-on-demand: a run pulls files from the network
  into `~/.cache/Tectonic` as needed, then reruns are offline. CI must
  warm the cache (or skip, like `tests/latex_goldens.rs`) — never let a
  bundle re-resolve silently change the index hash.

## Determinism pins (apply to BOTH oracle and candidate)

- `SOURCE_DATE_EPOCH=0`
- Engine `build_date` = UNIX_EPOCH (`crates/engine_xetex/src/lib.rs:187`)
- xdvipdfmx `deterministic_tags = true`
  (`crates/engine_xdvipdfmx/src/lib.rs:40`)
- `LC_ALL=C`, `TZ=UTC`
- single-threaded engine invocation (one process per document)
- run each document from a controlled CWD with a relative input name

## Host (shaping is version-sensitive)

Record the host where goldens were generated, since ICU/HarfBuzz/
FreeType versions affect shaping. Oracle and candidate share the
vendored bridge crates and must be built/run on the same host class.

- Generated on: <fill in: OS, arch> (e.g. Linux x86_64, WSL2)
- Cross-platform parity is out of scope for this corpus.

## Available bundle fonts used by the authored Tier 1 docs

Confirmed present in the pinned bundle index:
- Latin Modern OTF (`lmroman*-*.otf`)
- Libertinus (`LibertinusSerif/Sans/Mono`, `LibertinusMath-Regular.otf`)
- Amiri (Arabic, `Amiri-*.ttf`)
- Fandol (CJK, `Fandol*-*.otf`)
- Shobhika (Devanagari, `Shobhika-*.otf`)
- Noto (`NotoSans-*.ttf`, broad Unicode coverage)
