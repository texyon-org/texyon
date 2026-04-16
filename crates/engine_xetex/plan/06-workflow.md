# 06 — Per-session workflow

This document is the **concrete, mechanical checklist** for a working
session. Since the model loses memory between sessions, every session
follows the exact same shape. Read this to the end before you touch any
code.

## Step 0 — Orient (5 minutes)

1.  `cat plan/SESSION_LOG.md` — read at least the last two entries. Skim
    the one before that if it's short.
2.  `cat plan/MODULE_STATUS.md` — this is the source of truth for which
    C files / functions are still in C vs. Rust.
3.  `cat plan/PROGRESS.md` — know the current test pass count. That's
    your floor; it must not drop.
4.  `git log --oneline -20` — see what the previous sessions committed.

If any of those files disagree with `git log`, trust `git log` and update
the docs at the start of your session.

## Step 1 — Establish the baseline

Run the parity harness:

```
cargo build -p tectonic_engine_xetex
cargo test -p tectonic --test tex-outputs 2>&1 | tee /tmp/parity-before.txt
```

(Or, if a scoped test script exists: `./plan/scripts/parity.sh before`.)

Record the pass count. If it's lower than what `PROGRESS.md` claims,
**stop and investigate**. Either a previous session committed a regression
(fix first) or `PROGRESS.md` is stale (update it).

## Step 2 — Pick a target

Open `MODULE_STATUS.md` and pick a function-group from the top of the
"Next up" queue. Prefer:

-   Small functions (≤50 LOC)
-   Functions with few state dependencies (grep the C source for
    `extern`-referenced globals the function reads/writes)
-   Functions in the same file as work from the previous session (locality
    helps)
-   Functions with clear golden-test coverage

If nothing in the queue looks good, refer to `04-migration-order.md` for
the preferred next group.

## Step 3 — Read the C

Before writing any Rust:

1.  `Read` the entire C function. Not just the signature — the whole body.
2.  Note the globals it reads and writes. For each, either:
    -   The global already has a Rust accessor in `engine_xetex::globals`
        (use it)
    -   Or: add the accessor in a minimal, obvious way before doing
        anything else
3.  Note any `goto` labels. Plan how they become a loop + `break`/`continue`,
    or an `Option` return, or a small state enum. Don't use
    `labels_in_blocks` contortions.
4.  Note any macros used (`LLIST_link`, `BOX_width`, …). Each of these
    must already have a Rust counterpart on `Mem` / a typed view. Add it
    if missing.
5.  Identify *everything* the function calls. If any callee is still in
    C, the Rust version calls it via `extern "C"`. That's fine — it's
    what the incremental strategy buys us.

## Step 4 — Write the Rust

Place the translation in the module the file has been assigned to in
`01-architecture.md`. If the assignment is unclear, default to:
`src/<snake_case_of_c_file_without_xetex_prefix>.rs`. Example:
`xetex-scaledmath.c` → `src/scaled_math.rs`.

Every translated function MUST start with:

```rust
// translated from xetex-xetex0.c:NNNN (c_function_name)
```

where `NNNN` is the starting line number in the C source. This comment
is how we cross-reference during code review and how future sessions
verify correctness.

Expose the function with the same C ABI if it still has C callers:

```rust
#[no_mangle]
pub unsafe extern "C" fn get_node(s: i32) -> i32 {
    // ...
}
```

Once all C callers are gone, remove `extern "C"` + `#[no_mangle]` and
change the signature to whatever is idiomatic (`&mut Mem`, `MemIdx`, …).

### Special case: `static` C functions

A `static` C function has no linker-visible symbol, so
`#[no_mangle] extern "C" fn foo` in Rust cannot replace it — the
C callers find their file-local definition. Two legal resolutions:

1.  **Translate caller + callee in the same commit.** The Rust caller
    calls the Rust `foo` directly; the C caller is gone. This is the
    preferred route for small clusters (e.g. one external function and
    its one or two private helpers).

2.  **Un-static the C helper in place, as a prep commit.** Delete the
    `static` keyword on the C definition *and* declare the function
    in the header it belongs to. Verify the build still passes. Then
    the next commit translates the function the normal way.

Do NOT paper over this by writing a new non-static C wrapper that
calls the static — that's dead code the moment the function is in
Rust, and creates a confusing three-way dance.

## Step 5 — Remove the C

In the C source file, delete the body of the function but leave its
declaration reachable from headers for now. (Callers still find the
symbol — it's just provided by Rust.)

Then `cargo build -p tectonic_engine_xetex`. Fix link errors.

## Step 6 — Test

```
cargo test -p tectonic --test tex-outputs 2>&1 | tee /tmp/parity-after.txt
diff /tmp/parity-before.txt /tmp/parity-after.txt
```

Compare pass counts. **If the after-count is lower, you must not
commit.** Diagnose:

1.  Re-read the C function and the Rust translation side-by-side.
2.  Are you off-by-one on a Pascal 1-based index? Extremely common —
    TeX uses 1-based indices and unadjusted conversions are a classic
    bug.
3.  Did you miss a side-effect on a global?
4.  Did you convert a `goto` into something with different fall-through
    semantics?
5.  Did you change the order of two side-effects?

If you cannot diagnose within ~20 minutes, **revert your change**, leave
a note in `SESSION_LOG.md` describing what you tried and what you saw,
and pick a different target. Do not commit broken code; do not invent
a hack to make the test green.

## Step 7 — Commit

Small commits. Format:

```
engine_xetex: translate <c_function_name> to Rust

Translated from xetex-xetex0.c:NNNN. No behavior change; parity tests
still pass at <count>/<total>.
```

One function per commit when practical.

## Step 8 — Update docs

**This is not optional.** Before ending the session:

1.  Update `MODULE_STATUS.md`: mark the function as MIGRATED, with the
    commit hash. Update the per-file LOC and function-count remaining.
2.  Append an entry to `SESSION_LOG.md` following the template at the
    top of that file. Include: commit hash(es), parity count before
    and after, any surprises, anything the next session should know.
3.  Update `PROGRESS.md`: current pass count, current % of functions
    migrated.
4.  If you hit any new insights about idiom, state shape, or a
    translation gotcha, append a one-liner to the relevant design doc
    (`02-state.md`, `03-memory-and-nodes.md`, etc).

## Step 9 — When to stop for the session

Stop *before* your context runs out, not after. Leave 10–15% of budget
for docs updates. A session that translates 5 functions and documents
them all cleanly is more valuable than one that translates 10 and leaves
them un-documented.

## Session-recovery recipe (when things look wrong at start)

If you open a session and things are in a weird state:

1.  `git status` — are there uncommitted changes? Inspect, decide to
    keep or stash.
2.  `git log --oneline -5` — last commits. Does the state match?
3.  `cargo build -p tectonic_engine_xetex` — does it compile?
4.  Re-run parity — does the pass count match `PROGRESS.md`?
5.  If you find drift, your *first* commit of the session must be to
    reconcile the docs with the code. Do not start new translation
    work on a broken foundation.
