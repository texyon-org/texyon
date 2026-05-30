# 05 — Error handling & misc

> Status: **PLANNED (seed only — not READY).** This file currently holds
> the *target* and the reference implementation, captured so they aren't
> lost; it is **not** an execution recipe. Per hard rule 4 (`README.md`),
> nothing here may be executed until this plan is drafted to READY. Draft
> it after Phase C (workstream 01) and the FFI/de-unsafe teardown
> (workstream 03) have removed the C/C++ and thread-local bridge that
> currently block the clean shape.

This workstream reshapes the engine's error/abort flow and cleans up
dead-reference comments. It is the **lowest-priority, most-invasive-for-
least-parity-safety** workstream (`00-overview.md`), and is partly
optional — but it has a concrete, proven target.

## 1. The target error model = `engine_bibtex`

`crates/engine_bibtex` is a sibling Tectonic engine that has **already**
been translated to fully idiomatic Rust. It is the **reference
implementation** for this workstream's end-state. Verified (session 62):

-   **No escape mechanism.** `grep -rE 'setjmp|longjmp|_tt_abort'
    crates/engine_bibtex/src` → nothing. No `.c` files, no `cc` build
    step. The C error escape was *deleted*, not relocated.
-   **Fatal error is a `Result`.** `enum BibtexError { Fatal, Recover,
    NoBst }`, propagated with `?` to the top. A code comment states the
    intent directly: the classic TeX "fatal error" outcome "is
    represented as an `Err` result rather than a [`BibtexOutcome`]."
-   **`History` is a returned value**, not a global mutated before a
    longjmp: `enum History { Spotless, WarningIssued(u32),
    ErrorIssued(u32), FatalError }` flows back out of `bibtex_main`.
-   **State is stack-owned and `&mut`-threaded.** `bibtex_main` builds
    the state structs (`StringPool`, `HashData`, `GlobalBuffer`, …) as
    locals and threads a `GlobalItems { buffers: &mut …, pool: &mut …,
    … }` bundle down the call graph. **No thread-local, no `with_state`
    bridge.** This is the literal Phase-C end-state that workstream 01 is
    climbing toward.
-   **Rust entry point.** It enters through bridge_core's Rust API —
    `CoreBridgeLauncher::with_global_lock(|state| …) ->
    Result<BibtexOutcome>` — **not** a C `tt_*_main` + `jmp_buf` shim.

So the xetex end-state is not hypothetical: replace `overflow` /
`confusion` / `runaway` / `error`'s longjmp-via-`_tt_abort` with a
`Result`-shaped flow returning a `History`, entered via
`with_global_lock`, with state threaded as `&mut` (the Phase-C output).

## 2. Where the C escape actually lives (not xetex's to delete alone)

The `setjmp`/`longjmp` is the shared `tectonic_bridge_core` engine-entry
/ abort contract, **not** xetex code:

-   `_tt_abort()` → `longjmp(jump_buffer, 1)` and
    `ttbc_global_engine_enter()` (the `setjmp` target) are defined in
    `crates/bridge_core/support/support.c`. `xmalloc`/`xcalloc`/`xrealloc`
    failures and the `ttstub_*` callbacks all `_tt_abort`.
-   bridge_core's module docs note setjmp/longjmp-across-FFI is Undefined
    Behavior — which is *why* the `setjmp` sits in a C frame
    (`xetex-engine-interface.c`'s ~10-line `tt_engine_xetex_main`) with no
    Rust `Drop`. Moving that shim into Rust does **not** remove the
    longjmp; it relocates the UB-prone call. So the C anchor is the
    correct quarantine for now (`01-state-and-memory.md` §7).

**Correction to an earlier informal claim:** removing xetex's longjmp does
**not** require a new bridge_core ABI — the Rust `with_global_lock` entry
already exists and is proven by bibtex. xetex switches to it once the
conversion is done.

## 3. Why xetex can't just copy bibtex yet (the real blockers)

These are scope/sequencing blockers, not missing-mechanism blockers:

1.  **C/C++ bibtex never had.** xetex links the shaping stack
    (harfbuzz / graphite / freetype / ICU) and `xetex-XeTeXOTMath.cpp`.
    The `Result` route *tolerates* external shaping libraries (they
    return values, no unwinding needed), but the bridge_core C
    alloc/abort callbacks (`xmalloc`/`ttstub_*` that `_tt_abort`) must
    become their Rust equivalents first. This is largely happening
    already as the arenas became Rust-owned `Vec`s (workstream 01).
2.  **Scale + mid-migration.** `overflow`/`confusion`/`error` fire from
    hundreds of deeply-nested call sites, and state is still behind the
    Phase-B thread-local bridge. Converting each abort to a `Result`
    return and threading state as `&mut` *is* Phase C (workstream 01) +
    the FFI/de-unsafe teardown (workstream 03). 05 is the final flip once
    those land, not a parallel effort.

The `ALLOC_FAILED` sentinel introduced for the threaded allocators (`01b`
§5, session 62) is consistent with this target: it is a transient artifact
of the `with_state` borrow, to be deleted when the allocators' callers are
threaded and the exhaustion path becomes a normal `Result`/`History`
return rather than a wrapper-driven `overflow()` abort.

## 4. Misc (to flesh out when drafting to READY)

-   Dead-reference-comment cleanup (`translated from …` lines whose
    targets have moved, stale `goto`-label notes, etc.).
-   `error()` (the recoverable interactive handler) vs. `confusion()`
    (fatal internal-consistency abort): only reshape where it does **not**
    change observable behavior — TRIP + format goldens are the guard.
-   Decide the typed payload for fatal aborts (a `History`-like enum vs.
    `anyhow`) consistent with the bibtex `BibtexError`/`History` split.

## 5. Open questions for the READY draft

-   Can the threaded allocators eventually return `Option<i32>` /
    `Result<i32, Overflow>` (replacing `ALLOC_FAILED`) without forcing an
    `Option` cascade through every `new_*` constructor before its callers
    are threaded? (Likely: introduce it at the boundary last, once the
    constructors and their callers are threaded.)
-   Sequencing vs. workstream 03: the `#[no_mangle]`/`extern "C"` teardown
    and the `Result` reshape touch the same signatures — coordinate so a
    function is threaded, de-`unsafe`'d, and Result-shaped together rather
    than in separate passes.
