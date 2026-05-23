# Idiomatization of the XeTeX engine — Plan (Stage 2)

The pure C→Rust translation (Stage 1, see the parent `plan/` directory)
is **complete**: parity holds at 29/29 tex-outputs, 2/2 TRIP, 7/7
engine_rust, 4/4 latex_goldens, and every TeX algorithm body has been
translated. What remains in C is `xetex-macos.c` (out of scope, not
compiled on Linux), the `xetex-engine-interface.c` `setjmp` entry shim
plus its aarch64 linker hack, two tiny `yhash` glue helpers, and static
data/prototype stubs.

`xetex-macos.c` is explicitly **not part of Stage 2**. It cannot be
validated in the current environment, so do not translate it and do not
let macOS-only state references block a Linux-validated workstream.

But the *result* of Stage 1 is faithful, not idiomatic. The code is
"Pascal-in-Rust": 384 `extern static mut` globals behind a 5,569-line
`globals.rs`, 576 `#[no_mangle]` exports, 694 `unsafe fn` items,
hundreds of raw `unsafe { ... }` blocks, raw `i32` indices into
`mem[]`/`eqtb[]`, and single files as large as 18,748 lines
(`scan.rs`).

**Stage 2 makes the engine idiomatic Rust without changing its
behavior.** This directory holds the long-term plan for that work.

**Like Stage 1, this work spans many sessions with the model reset
between each.** Two distinct activities live here, and you may be doing
either one in a given session:

1.  **Plan construction** — writing and refining the detailed workstream
    plans in this directory. (That is what early sessions do.)
2.  **Plan execution** — actually refactoring the crate per a plan that
    is marked `READY` in `STATUS.md`.

Every document is written to be read cold. If you are a new session,
read in this order:

1.  [`00-overview.md`](00-overview.md) — the diagnosis (why the code is
    stuck at "Phase A"), the end-state target (full Phase C, decided
    2026-05-23), the guardrails, and the workstream map + sequencing.
2.  [`STATUS.md`](STATUS.md) — the authoritative tracker for **both**
    plan-construction progress and execution progress. Read this to
    know what to do next.
3.  [`SESSION_LOG.md`](SESSION_LOG.md) — running log; read the top
    (newest) entry first for the handoff.
4.  The numbered workstream plans (`01-…`, `02-…`, …) — read the one
    you are about to construct or execute. **Execution sessions: read
    [`01-state-and-memory.md`](01-state-and-memory.md) §11 ("Execution
    recipe") — the step-by-step procedure (grep checks, accessor
    repoint, cleanup, validation, commit) plus the recurring gotchas.**

## Relationship to the Stage 1 plan (`../`)

The parent `plan/` documents already describe much of the *target*:

-   [`../01-architecture.md`](../01-architecture.md) — the target crate
    layout (`state/`, `node/`, split modules). Stage 2 executes it.
-   [`../02-state.md`](../02-state.md) — defines Phases A/B/C of state
    migration. Stage 1 stopped at Phase A. Stage 2 = Phases B and C.
-   [`../03-memory-and-nodes.md`](../03-memory-and-nodes.md) — the
    `MemIdx` / `Scaled` / typed-accessor design. Stage 1 never built it.
    Stage 2 does.
-   [`../07-anti-patterns.md`](../07-anti-patterns.md) — **still in
    force.** Every anti-pattern that governed translation governs
    refactoring too, especially A.4 (no clone-to-silence), A.5 (no
    `Box<dyn Node>`), A.6 (no speculative generics), and A.7 (never
    rewrite Knuth's algorithms).

Stage 2 does not invent a new target; it realizes the one those docs
already sketched. Where this directory and a parent doc disagree, this
directory is newer and wins — but note the disagreement in
`SESSION_LOG.md` so the parent can be reconciled.

## Hard rules (violation blocks the session)

1.  **Never regress the test dashboard.** All four suites
    (`tex-outputs` 29, `trip` 2, `engine_rust` 7, `latex_goldens` 4)
    must pass before and after every commit. Idiomatization is a
    *behavior-preserving* refactor: the bytes out of the engine do not
    change. If output changes, you changed behavior — revert.
2.  **No algorithm changes.** This is refactoring, not rewriting. The
    line-breaker, page-builder, and math layout stay
    line-for-line-equivalent. (Anti-pattern A.7.)
3.  **Small, reversible commits.** One coherent refactor per commit so
    a parity regression bisects to a single change. A 30-function
    untested commit is forbidden (A.12).
4.  **Plan before executing a workstream.** Do not start refactoring a
    workstream whose plan doc is not marked `READY` in `STATUS.md`.
    Construction sessions produce `READY` plans; execution sessions
    consume them. This separation is what keeps the work bisectable
    across model resets.
5.  **Always update `STATUS.md` and `SESSION_LOG.md` at session end.**
    Future you has no memory of this one. (A.11.)
6.  **Do not translate `xetex-macos.c` in this plan.** Treat it as
    platform glue outside the validated surface unless the project later
    adds a macOS parity environment and an explicit new plan.
