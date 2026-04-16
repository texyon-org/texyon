# 00 — Strategy & Principles

## Why we're doing this

Tectonic's `engine_xetex` crate currently wraps the original XeTeX C
implementation. The C code is not Web-generated Pascal — it is a hand-edited C
translation of the Pascal — but it is still written in a style that is
idiomatically Pascal: global `extern` variables, raw integer indices into a
huge shared `mem[]` arena, macro-defined "struct field" accessors, and
unchecked implicit invariants everywhere. This makes it hard to extend
(custom primitives, new math features, HTML-oriented output, alternative
paragraph algorithms, …) without fear of breaking something at a distance.

The goal is **a clean, idiomatic Rust engine that preserves XeTeX's
semantics and performance**, so that future extensions can be made with
confidence.

## The two failure modes we must avoid

There was a previous attempt at this, which failed. Its two lessons shape
everything in this plan:

### Failure mode 1: "replaying history"

The previous attempt started from `tex.web` (the original Knuth TeX source),
translated that to Rust to get a tex82-equivalent, and then planned to layer
e-TeX and XeTeX features on top. This caused years of work to be done twice:
every line of tex.web was touched again in the e-TeX and XeTeX C patches, and
the modern engine has decades of cumulative fixes that would have had to be
re-applied.

**Fix:** this time, we start from **the `xetex-*.c` files that are
currently in this crate**. They already contain all of XeTeX's features and
bug fixes. The task is pure translation C → Rust. No history replay.

### Failure mode 2: "output-chasing hacks"

The previous attempt, once its `main_control` loop could compile real
documents, tried to make the DVI output byte-exact with `initex` by feeding
diffs back to the model and asking it to fix the output. The model, lacking
the ability to reason about non-local effects, started adding ad-hoc branches:
*"if the macro currently being executed is `\foo`, behave differently"*,
*"if the current byte offset in the output file is in range X..Y, emit Z
instead"*. Each patch made one test pass and broke an invariant elsewhere.
The codebase became unrecoverable.

**Fix:** the root cause was *translating speculatively from a spec*. This
time we translate *from working C code*. Every Rust function has a direct
C referent: if its output diverges, the bug is a miss-translation in *that
function*, and the fix is to re-read the C. Output-chasing with ad-hoc
branches is **forbidden** (see `07-anti-patterns.md`).

And critically: we translate **incrementally, one function (or small
function-group) at a time, running the full test suite after each change**.
Divergence is caught within one function's blast radius.

## The core strategy: incremental function-level replacement

The C engine is already wired into the Rust crate via FFI. We use a
**"Ship of Theseus"** approach:

1.  Rust and C coexist in the crate throughout the transition.
2.  We pick one C function, write a Rust replacement, expose it via
    `#[no_mangle] extern "C"`, remove the C definition, and keep the
    `extern` declaration so call sites still link.
3.  After each replacement, we run the full test suite (`cargo test -p
    tectonic --test tex-outputs` at minimum). **Pass count must not
    regress.**
4.  Commit per function or small group. This gives us a clean bisection
    history and a short rollback distance.
5.  Global C variables stay as `extern` symbols during this phase. The
    Rust code reads and writes them through small typed wrappers
    (`mod globals`).
6.  Once enough functions are in Rust that a given C global is only
    touched from Rust, we migrate that global into an `EngineState`
    struct — one at a time — by introducing a state parameter to the
    Rust functions that touch it and removing the `extern`. (Details
    in `02-state.md`.)
7.  When the last C function is removed, the `xetex/` directory can be
    deleted and the crate is pure Rust.

This strategy has three properties that make it robust:

-   **Every commit is a working engine.** You can run real documents
    through it at any time.
-   **Bugs are strictly local.** If tests regress after replacing
    function `f`, the bug is in the translation of `f`.
-   **Progress is measurable.** The test-pass count and the count of
    functions still in C are two numbers that monotonically improve.

## Principles for the Rust code

1.  **Clean, idiomatic Rust.** Use types, iterators, `match`, `Option`,
    `Result`. Do not transliterate `goto`, `int32_t` placeholders, or
    Pascal `with`-blocks. Do not preserve 1-based indexing.
2.  **No gratuitous clones or allocations.** TeX is performance-sensitive,
    especially in the inner tokenizer / expansion / paragraph-breaking
    loops. Borrow where possible; use indices (`Mem Idx`, `EqtbIdx`, …)
    into arenas instead of `Rc`/`Arc`. Profile before optimizing, but
    never write code whose obvious form is "clone the world".
3.  **No premature abstractions.** A direct translation with newtype
    indices is fine. We do *not* need trait objects, generic nodes, or
    an ECS until (and unless) a concrete need forces it.
4.  **Preserve memory locality.** TeX's speed comes from `mem[]` being
    a single flat array that the CPU prefetcher loves. The Rust side
    keeps flat `Vec`-based arenas. `Box<Node>` per node is forbidden.
    (Details in `03-memory-and-nodes.md`.)
5.  **Unsafe is an escape hatch, not a habit.** We'll need it at the FFI
    boundary and possibly for the memory_word bit-field tricks during
    the transition. Every `unsafe` block gets a `// SAFETY:` comment
    explaining the invariant.
6.  **Comments explain *why*, not *what*.** In particular, every
    translated function gets a one-liner pointing at its C origin:

    ```rust
    // translated from xetex-xetex0.c:206 (get_node)
    pub fn get_node(state: &mut EngineState, s: i32) -> MemIdx { … }
    ```

    This is the thread by which future sessions reconstruct intent.
7.  **Panics are bugs, not control flow.** TeX has an elaborate
    error-recovery system (`error()`, `confusion()`, …). Translate it
    to `Result` or to engine-internal error types, not to
    `panic!`/`unwrap()`. Panics may only arise from contract violations
    that indicate the engine itself is broken.

## Non-goals

-   **Byte-identical output with original TeX Live xetex.** Tectonic's
    C xetex is already not byte-identical with upstream xetex in all
    cases (different font resolution, different bundle system, …). Our
    goal is byte-identical output with *this crate's current C code*,
    as measured by the existing test suite.
-   **New user-visible features.** This is pure refactoring. New
    primitives, extensions, and HTML output work happen *after* the
    rewrite is complete, on a clean Rust base.
-   **Format-file compatibility with other TeX engines.** We already
    have our own format serial (`FORMAT_SERIAL = 33`). We're free to
    change it. We do, however, need the Rust engine to be able to
    consume format files that the *same* engine produces in initex
    mode, so in-tree plain.fmt etc. still work round-trip.
