# PGO build of the CLI

`tools/pgo/pgo.sh` builds a profile-guided optimized `texyon` binary in three
steps:

1. **gen**: release build with `-Cprofile-generate` (instrumented).
2. **train**: compile every document in `train.txt` with the instrumented
   binary, then merge the raw profiles with `llvm-profdata`.
3. **use**: release build with `-Cprofile-use`.

```sh
rustup component add llvm-tools   # once, provides llvm-profdata
tools/pgo/pgo.sh                  # gen + train + use
target/pgo/use/x86_64-unknown-linux-gnu/release/texyon --version
```

Stages can be run on their own (`pgo.sh gen|train|use`), extra arguments are
passed to `cargo build`. `PGO_TARGET_DIR` moves the build output (default
`target/pgo`), `PGO_PROFILE=<file>` skips training and uses a stored profile,
`PGO_WARN=1` makes LLVM list functions that got no profile data.

Notes:

- `llvm-profdata` must come from the same LLVM major as `rustc`
  (`rustc -vV`). The script uses the rustup `llvm-tools` copy and refuses a
  mismatch. Distro `llvm-profdata` is usually older and cannot read the
  profiles.
- Only Rust code is instrumented and optimized. The C/C++ parts built through
  `cc` (harfbuzz, graphite2, freetype, libpng, teckit, xdvipdfmx/pdf_io) use the
  normal flags in both builds. Instrumenting them would need a clang of the
  same LLVM major as rustc. Engine time is almost all Rust macro expansion.
- `--target <host>` is passed so the flags skip build scripts and proc macros,
  so the output lives under `<target-dir>/<host>/release`.
- The profile is tied to the source. After code changes, retrain: a stale
  profile still builds but silently loses most of the benefit.
- Training inputs live in `train.txt`. Keep them disjoint from documents used
  to benchmark the result. `corpus:` entries come from
  `crates/engine_xetex/scripts/corpus_fetch.sh` and are skipped when absent.
- Output stays byte-identical, check with
  `TEXYON=<pgo binary> crates/engine_xetex/tests/upstream/run.py`.

Wasm: stable rustc ships no profiler runtime for `wasm32-wasip1`, so the
module cannot be instrumented without `-Zbuild-std` and a self-built
compiler-rt. Reusing this native profile for the wasm build does not work
directly either, because crate hashes in symbol names differ per target. With
names remapped it still only gives about 2-3%, since the hottest engine
functions fail the CFG hash check on wasm.
