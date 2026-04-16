# Rustification of the XeTeX engine — Plan

This directory holds the long-term plan for converting the `engine_xetex` crate
from its current C implementation (in `xetex/*.c`, ~50k lines) to idiomatic,
clean, performance-preserving Rust.

**This work will span many sessions, with the model being reset between each
one.** Every document in this directory is written to be self-contained and
readable cold. If you are a new session starting work, read in this order:

1.  [`00-overview.md`](00-overview.md) — strategy, principles, why we're
    doing this, why the previous attempt failed.
2.  [`07-anti-patterns.md`](07-anti-patterns.md) — **read this before
    writing any code.** The previous attempt failed by way of specific
    anti-patterns; this document enumerates them.
3.  [`06-workflow.md`](06-workflow.md) — step-by-step workflow for a
    single session. How to pick up where the previous session left off,
    how to move one function across, how to record progress, how to end
    a session.
4.  [`SESSION_LOG.md`](SESSION_LOG.md) — the running log of what has
    happened in prior sessions. Read the last entry first.
5.  [`MODULE_STATUS.md`](MODULE_STATUS.md) — the authoritative,
    machine-readable status of every C source file being rewritten.
6.  [`PROGRESS.md`](PROGRESS.md) — the test-parity dashboard.
7.  The remaining design documents (`01`–`05`) are reference material:
    read them when you need them.

## Reference documents

-   [`01-architecture.md`](01-architecture.md) — target Rust crate
    layout and the incremental FFI-interleaving strategy.
-   [`02-state.md`](02-state.md) — how global TeX state (mem, eqtb, save
    stack, input stack, …) will be modeled and migrated.
-   [`03-memory-and-nodes.md`](03-memory-and-nodes.md) — the arena /
    node-typing plan. Preserves TeX's memory locality.
-   [`04-migration-order.md`](04-migration-order.md) — the dependency
    order in which C files should be moved to Rust.
-   [`05-testing.md`](05-testing.md) — the parity harness, golden-test
    strategy, and unit-test conventions.

## Hard rules (violation blocks the session)

1.  **Never add a behavioral branch whose predicate is "what macro is
    being executed" or "what byte offset we're at in the output stream"
    or any other context-sniffing hack.** If tests diverge, the bug is
    in a function you translated; go re-read the C.
2.  **Never add code that is not a direct translation of an identifiable
    construct in the C source.** Every non-trivial Rust function has a
    comment citing its C origin (`// translated from xetex-xetex0.c:NNNN`).
3.  **Never regress the test parity dashboard.** If `cargo test -p
    tectonic --test tex-outputs` passes N cases before your change, it
    must pass ≥N cases after. New passes are celebrated; new failures
    block the commit.
4.  **Never hand-write "clones everywhere" Rust.** See the performance
    clauses in `00-overview.md`.
5.  **Never skip updating `MODULE_STATUS.md` and `SESSION_LOG.md` at the
    end of a session.** Future you has no memory of this one.
