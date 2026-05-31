# 04 — Migration order

The dependency-ordered list of what to translate, and why in that
order. Follow the phases top-to-bottom. Within a phase, you can
reorder based on the "smallness" heuristic — smaller first.

## Phase 0 — Scaffolding (before any translation)

1.  Session 1 establishes the parity baseline. `cargo test -p tectonic
    --test tex-outputs` is recorded.
2.  Create `src/globals.rs` with `extern "C"` bindings for every
    global declared in `xetex-xetexd.h`. Use a script:

    ```sh
    grep -E '^extern [a-zA-Z_].*[a-zA-Z_0-9*]+\s*(\[|;)' \
         crates/engine_xetex/xetex/xetex-xetexd.h | sort > /tmp/globals.txt
    ```

    Hand-translate each. Group by cluster (see `02-state.md`). Write
    safe accessor functions for the ones used in Phase 1.
3.  Create `src/mem_types.rs` with `MemoryWord`, `B32x2`, `B16x4`
    as defined in `03-memory-and-nodes.md`. Add a compile-time
    assertion that `size_of::<MemoryWord>() == 8` (or whatever the C
    side has).
4.  Create `src/mem.rs` with `MemIdx`, `Scaled`, `FontId`,
    `StrNumber`, `Token` newtypes. No accessors yet.

## Phase 1 — Pure helpers (target: `src/scaled_math.rs`, `src/helpers.rs`)

**Why first:** these functions have zero or near-zero state access.
They are the "hello world" of this migration. They validate the
build/test/commit loop on low-risk code.

**Source files to mine:** `xetex-scaledmath.c` (513 LOC, all pure
math), plus a handful of pure helpers scattered through
`xetex-xetex0.c` (`badness` at line 25, `half`, random-number
helpers, etc).

**Expected deliverables:**
-   Every function in `xetex-scaledmath.c` translated.
-   `badness()` and `half()` from `xetex-xetex0.c` translated.
-   Unit tests for each, written in Rust (`#[cfg(test)]`),
    regression-testing edge cases the C code handles.

**Hard part:** none. If Phase 1 is hard, something is wrong with the
scaffolding — go back to Phase 0.

## Phase 2 — Memory / node allocation (target: `src/mem.rs`)

**Why next:** almost every translation in later phases calls
`get_node`, `free_node`, `new_null_box`, etc. Getting them into Rust
unlocks everything downstream.

**Source:** `xetex-xetex0.c:170–454` (all the `get_*`, `free_*`,
`new_*` functions). Roughly 20 functions.

**Translation rules:**
-   Take `&mut Mem` (Phase B) — but in Phase A, read/write the C
    globals for now. Add the `&mut Mem` parameter later.
-   Add typed `NodePtr<K>` handles and field methods (see
    `03-memory-and-nodes.md`) as you need them. Don't preemptively add
    node APIs for nodes you haven't touched yet. Transitional raw
    `MemIdx` field helpers are allowed only as bridge/private code.
-   Preserve the C code's invariants about the free-list rover: it
    walks in a specific order, and the order *does* affect which
    word is reused next, which can affect output in subtle ways
    (specifically, the order of shared glue-spec allocations).

**Expected deliverables:**
-   ~20 allocation functions in Rust.
-   Typed accessors on `NodePtr<K>` handles (e.g. `p.width(mem)`).
-   `NodeType` enum with all variants.

## Phase 3 — Printing and error plumbing (target: `src/output.rs`, `src/errors.rs`)

**Why here:** the error-recovery machinery (`error`, `confusion`,
`overflow`, `fatal_error`) is called from almost every interesting
function. Until it's in Rust, every Rust function that might fail
has to `extern "C"` call into C to report. That's fine but
awkward; moving it over early reduces friction.

**Source:** `xetex-output.c` (768 LOC, all `print_*` and
`show_*`), `xetex-errors.c`.

**Gotchas:**
-   Printing goes through `selector` which picks between stdout,
    log file, terminal, pseudo-file, etc. Don't use `println!` or
    Rust's `Write` trait at first — call through the same bridge
    I/O as the C code via `rust_output_handle_t`.
-   `print_int`, `print_scaled`, etc have subtle formatting that
    must match byte-for-byte in the `.log` files. The parity tests
    check `.log` contents.

## Phase 4 — Strings (target: `src/string_pool.rs`)

**Why:** the string pool (`str_pool`, `str_start`) is a single
simple data structure used by errors, I/O, and format files.
Completing it unblocks a lot.

**Source:** `xetex-stringpool.c` (206 LOC), plus related helpers in
`xetex-xetex0.c`.

**Plus:** migrate the `StringPool` cluster to Phase B (see
`02-state.md`) — it's small enough to serve as the first cluster
migration and prove the pattern.

## Phase 5 — Scanner, tokenizer, expand (target: `src/scanner.rs`, `src/expand.rs`, `src/token.rs`)

**This is the largest phase.** Roughly 5,000 LOC of `xetex-xetex0.c`
is the tokenizer and expansion machinery.

**Strategy:**
-   Translate in dependency order inside the phase:
    `get_x_token` → `get_token` → `get_next` → `scan_keyword` →
    `scan_int` → `scan_dimen` → ... → `macro_call` → `expand` →
    `conditional`.
-   Do NOT translate `main_control` yet — it's the dispatcher and
    it imports everything.

**Gotchas:**
-   The tokenizer reads `cur_input` and mutates it in place. This
    is a classic spot for aliasing pain. Encapsulate early with a
    `Tokenizer<'a>` struct that holds `&mut InputStack` and
    `&mut StringPool`.
-   `\csname`/`\endcsname` and `\expandafter` have non-obvious
    control flow. Translate carefully from the C; the C code is
    correct.

## Phase 6 — Page builder & line breaker (target: `src/pagebuilder.rs`, `src/linebreak.rs`)

**Source:** `xetex-pagebuilder.c` (804 LOC), `xetex-linebreak.c`.

**Gotchas:**
-   The K&P line breaker is not rewrite-bait. Translate verbatim.
-   Active/passive/delta nodes use the accessor macros from
    `xetex-xetexd.h:220–235`. Add the corresponding `Mem` methods.
-   `\output` routine interaction: the page builder can invoke
    `main_control` recursively through `\output`. Keep the state
    passing compatible with that.

## Phase 7 — Math layout (target: `src/math.rs`)

**Source:** `xetex-math.c` (3,833 LOC).

Self-contained-ish. Depends on Phase 2 (allocation), Phase 3
(printing/errors), and Phase 5 (scanning delimiters).

## Phase 8 — Shipout (target: `src/shipout.rs`, `src/synctex.rs`)

**Source:** `xetex-shipout.c` (2,451 LOC), `xetex-synctex.c`
(1,283 LOC).

**Why late:** shipout is what generates DVI/XDV output and is the
place where any bug in earlier phases becomes visible. Translating
it last means you are translating it against a correct Rust
implementation of everything else, so divergence in shipout's own
output pinpoints shipout bugs.

## Phase 9 — Fonts, extensions, pictures, I/O

**Source:** `xetex-ext.c`, `xetex-pic.c`, `xetex-io.c`,
`xetex-texmfmp.c`, `xetex-engine-interface.c`.

Some of this (the ext.c native-font loading) crosses into
`tectonic_xetex_layout`. Don't translate xetex_layout — that's
a separate crate. Do translate the glue code that calls into it.

## Phase 10 — Main control & initialization

**Source:** the `main_control` function (big dispatch loop in
`xetex-xetex0.c`) and `xetex-ini.c`'s `tt_engine_xetex_main` /
init/allocation code.

**Why last:** `main_control` is the thing that turns everything else
into an actual running engine. By now, every function it dispatches
to is already Rust, so translating it is mechanical: rename labels
to match labeled `loop {}` blocks, fold `goto` into breaks, use
`match` for the opcode dispatch.

## Phase 11 — Format file I/O

**Source:** `load_fmt_file`, `store_fmt_file`, and the `xetex-swap.h`
byte-swap helpers.

These are translated as a pair so we have a round-trip test. Do not
attempt to read a format file produced by an older version of the
engine.

## Phase 12 — C teardown

-   Remove `build.rs` (no more C to compile).
-   Remove the `xetex/` directory.
-   Remove `cbindgen.toml` if no C bindings are generated anymore.
-   Remove `tectonic_bridge_core`-internal FFI types from `c_api`.
-   Clean up the `extern "C"` wrappers around `#[no_mangle]` Rust
    functions that no longer need them.
-   Tag a release. Probably bump `FORMAT_SERIAL` since format-file
    layout may have drifted.

## Rough effort estimation

Not a schedule — just an order-of-magnitude sanity check so
sessions know when to worry.

| Phase | LOC C | Approx. sessions |
|-------|------:|-----------------:|
| 0     |     — |  1               |
| 1     |  ~800 |  2–3             |
| 2     |  ~700 |  3–5             |
| 3     | ~1100 |  4–6             |
| 4     |  ~400 |  2–3             |
| 5     | ~5000 |  15–25           |
| 6     | ~2500 |  8–12            |
| 7     | ~3800 |  10–15           |
| 8     | ~3700 |  10–15           |
| 9     | ~2500 |  6–10            |
| 10    | ~2000 |  4–8             |
| 11    |  ~800 |  2–3             |
| 12    |     — |  1               |
| **Total** | **~23k** | **~70–100 sessions** |

The total C is ~50k but half of that is whitespace, headers, and
comments, so ~23k is a rough "translatable LOC" number. "Sessions"
here is a very rough unit that assumes modest per-session output.
Real progress will vary; use these as sanity checks only.
