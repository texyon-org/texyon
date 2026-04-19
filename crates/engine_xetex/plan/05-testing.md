# 05 — Testing strategy

Correctness is measured at three levels:

1.  **Unit tests** (fast, local). Pure Rust functions get pure Rust
    tests via `#[cfg(test)]` modules.
2.  **TRIP conformance tests** (fast, engine-level). `cargo test -p
    tectonic --test trip` runs the Knuth TRIP and e-TRIP
    tests directly against the engine crate. Both `trip_test` and
    `etrip_test` must pass (2/2) at all times. These catch low-level
    TeX engine bugs that the slower parity suite might miss.
3.  **Parity tests** (slow, end-to-end). The existing
    `tests/tex-outputs.rs` suite compiles real `.tex` documents and
    compares generated `.xdv`/`.log`/`.pdf` against golden files
    byte-for-byte.

Both the TRIP tests and the parity tests are authoritative correctness
signals. Unit tests help localize bugs but cannot replace them.

## Running the TRIP tests

```
cargo test -p tectonic --test trip
```

Expected output: `test result: ok. 2 passed; 0 failed`.

The TRIP tests (`tests/trip.rs` in `crates/engine_xetex`) compare the
engine's output against golden TRIP log/xdv/fot files. They are fast
(~1 second) and should be run alongside the parity suite at every step.

## Running parity tests

The existing test runner is defined in `tests/tex-outputs.rs` at the
repository root. The canonical command is:

```
cd ~/Code/tectonic
cargo test -p tectonic --test tex-outputs
```

This command:
1.  Builds the tectonic workspace (including `engine_xetex`).
2.  Ensures the `plain.fmt` cached format is available (via
    `ensure_plain_format` in the test util).
3.  Runs each `TestCase` in `tests/tex-outputs/`. For each case,
    compiles the `.tex` through the engine, and compares the
    generated artifacts against the `*.xdv`, `*.log`, `*.pdf` files
    in that directory.

At session 0 there are ~25–30 distinct test cases. Per
`plan/PROGRESS.md`, every session records the pass count before and
after its changes.

## Parity pass count tracking

Parse the test runner output at the end of every session. A simple
oneshot:

```sh
cargo test -p tectonic --test tex-outputs 2>&1 \
    | grep -E '^test result:' \
    | tee -a plan/parity-history.log
```

The final line looks like:

```
test result: ok. N passed; M failed; ... 
```

Record `N` in `PROGRESS.md`. `M` must not increase from session to
session.

## The parity harness script (to be added in session 1)

A small helper lives at `plan/scripts/parity.sh`. Proposed contents:

```sh
#!/usr/bin/env bash
# Usage: plan/scripts/parity.sh [before|after]
# Runs the parity tests and records to plan/parity-history.log.
set -euo pipefail
label="${1:-run}"
ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
out="$(mktemp)"
cd "$(git rev-parse --show-toplevel)"
cargo test -p tectonic --test tex-outputs 2>&1 | tee "$out"
result="$(grep -E '^test result:' "$out" | tail -1)"
echo "${ts}  ${label}  ${result}" >> crates/engine_xetex/plan/parity-history.log
```

Session 1 creates this script and `chmod +x`es it. Session 2+
uses it rather than re-typing the `cargo test` invocation.

## Unit tests

Guidance:

-   **Pure math:** `scaled_math.rs`, `helpers.rs`, anything in
    Phase 1. Write unit tests at translation time, with hand-picked
    edge cases (zero, negatives, overflow, boundary values).
-   **Arena allocation:** Phase 2. Unit-test the allocator in
    isolation with a `Mem::new(1024)` fixture. Test:
    -   `get_avail` / `flush_list` round-trip (single-word free list)
    -   `get_node(k)` / `free_node` round-trip (variable-size free
        list)
    -   Rover advancement under fragmented allocation patterns
-   **Typed accessors:** for every `box_width` / `set_box_width`
    pair, a one-line round-trip test is fine.
-   **Tokenizer primitives:** small inputs, check `cur_cmd`/`cur_chr`
    transitions. Hard to do without a full input stack — may be
    deferred to Phase 5.

Rule of thumb: if a function has identifiable inputs and outputs
(i.e. is not simply mutating global state), write a unit test. If
it's pure state mutation, parity tests are the correctness check.

## Golden-file failure diagnosis

When a parity test fails, you will typically see a diff between the
generated and expected `.log` or `.xdv`. Steps:

1.  Look at the **first** difference, not the last. Later diffs are
    cascade effects of the first.
2.  For `.log` divergence: the difference is usually a print
    function (`print_int`, `print_scaled`, `print_glue`, …) or an
    error-recovery pathway that took a different branch.
3.  For `.xdv` divergence: the difference is in shipout or in
    something feeding shipout (box dimensions from the page
    builder, glue resolution from linebreak, …).
4.  **Before patching:** identify the most recently translated
    function in that execution path. Bisect by commit if unclear.
5.  Re-read the C for that function line-by-line next to your
    Rust. Do not patch blindly. See `07-anti-patterns.md` §A.1.

Helper: `xdvcmp` or a plain `xxd` diff works on the `.xdv`.
`tectonic` also has a `dump` utility that pretty-prints xdv opcodes;
check `crates/xdv/` for tooling.

## Tests we may want to add

As translation progresses and we hit edge cases not covered by the
existing golden set, we may want to add new golden cases. Rules:

-   Adding a golden test is not a license to commit broken output.
    The golden is generated by running the current (correct) engine
    at the point of adding the test, so it captures known-good
    behavior, not the new translation's output.
-   New goldens should be small `.tex` files exercising a specific
    primitive or construction.
-   Add them under `tests/tex-outputs/` with the existing
    convention: `stem.tex`, `stem.xdv`, `stem.log`.

## What to do when the whole suite is broken

If a session starts and parity is 0 / all failed:

1.  Something is badly broken. Do not attempt new translation.
2.  `git bisect` to find the introducing commit. Revert it if the
    fix is non-obvious.
3.  Note the incident in `SESSION_LOG.md` and reset the green-build
    streak in `PROGRESS.md`.

## Benchmarking (post-rewrite only)

During the migration we do not chase performance. The goal is
correctness parity; profiling is premature until the translation is
done. After Phase 12, we benchmark:

-   compile time for a long document (say, the Lua(La)TeX
    reference or a typeset book chapter)
-   memory high-water mark
-   cache-miss rate on the paragraph-breaker inner loop (`perf
    stat` is enough)

We measure against the C engine (still available on prior commits
of this repo or from upstream tectonic releases) and target
≥ parity. If the Rust engine is 10% slower, that's acceptable; if
it's 50% slower, we profile and fix before shipping.
