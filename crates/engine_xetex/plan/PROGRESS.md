# Progress dashboard

Live metrics for the C → Rust migration. Updated at the end of every
session in Step 8 of `06-workflow.md`.

## Test parity

The authoritative parity command (run from the repository root):

```
cargo test -p tectonic --test tex-outputs
```

| Session | Date       | Passed | Total | Delta | Notes                              |
|--------:|------------|-------:|------:|------:|------------------------------------|
|       0 | 2026-04-15 |      — |     — |     — | Plan scaffolding only. Baseline to be recorded in session 1. |
|       1 | 2026-04-16 |     29 |    29 |     — | Baseline recorded. See `plan/parity-history.log`. |
|       2 | 2026-04-16 |     29 |    29 |     0 | Translated 7 more scaledmath functions. |
|       3 | 2026-04-16 |     29 |    29 |     0 | badness, get_avail, flush_list, get_node, free_node. |

**Baseline rule:** session 1 must run the test suite *before* making
any Rust changes and record the (Passed, Total) pair as the "Session 0"
baseline. After that, no session may commit changes that drop the
Passed count below the previous session's recorded value.

## Code footprint

| Session | C LOC | Rust LOC (engine only) | C/(C+Rust) |
|--------:|------:|-----------------------:|-----------:|
|       0 |39,419 |                      0 |       100% |
|       1 |39,272 |                    ~220 |      ~99.4% |
|       2 |38,825 |                   ~1000 |      ~97.4% |
|       3 |38,674 |                   ~1530 |      ~96.2% |

(C LOC excludes teckit, xetex-XeTeXOTMath, and xetex-macos, which are
not in scope for translation. Measured with `wc -l` over the 16 in-scope
`.c` files listed in `MODULE_STATUS.md`. Rust LOC counts the engine's
own `src/*.rs` modules, excluding tests and `lib.rs`.)

## Functions migrated

| Session | Rust fns | C fns remaining | % migrated |
|--------:|---------:|----------------:|-----------:|
|       0 |        0 |             ~?? |         0% |
|       1 |        6 |           ~???? |     <0.5%  |
|       2 |       13 |           ~???? |      ~1%   |
|       3 |       18 |           ~???? |     ~1.5%  |

Total C function count to be measured in session 1 via:

```
for f in xetex/xetex-*.c; do
    echo "$(grep -c '^[[:alpha:]_][[:alpha:]0-9_ *]*\s*[[:alpha:]_][[:alpha:]0-9_]*\s*(' "$f") $f"
done | sort -nr
```

## Green-build streak

Count of consecutive sessions without a parity-count regression. A
regression resets this to zero and must be noted in `SESSION_LOG.md`.

Current streak: **3** (sessions 1–3 held parity at 29/29).

## Health checks

A session should fail fast if any of these are broken:

-   [ ] `cargo build -p tectonic_engine_xetex` succeeds
-   [ ] `cargo build -p tectonic` succeeds
-   [ ] `cargo test -p tectonic_engine_xetex` passes (unit tests inside the crate)
-   [ ] `cargo test -p tectonic --test tex-outputs` passes at ≥ the previous count
-   [ ] `cargo clippy -p tectonic_engine_xetex -- -D warnings` is clean (once we have non-trivial Rust)

If any box is unchecked at session start, **fixing the foundation is
the first task of the session**. You do not start new translation work
on a broken baseline.
