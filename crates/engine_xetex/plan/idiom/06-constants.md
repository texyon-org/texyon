# 06 — Constants & magic numbers

Replace the engine's pervasive bare numeric literals and per-module
duplicated `const`s with a single shared, **computed** constant module that
mirrors the WEB `@d` definition chains. This is a distinct idiomatization
gap not covered by workstreams 01–05.

> Status: **EXECUTING** (2026-05-30). Execute the
> low-risk "centralize duplicated cross-module consts" phase (§5 Phase A)
> independently; defer the deep literal sweep (§5 Phase B) to pair with
> Phase C / the newtype sweep.

## 1. Why this exists

Nothing in 01–05 targets numeric-literal cleanup, and clippy will not
catch it (there is no stable "magic number" lint; `unreadable_literal`
only enforces digit grouping). Yet the symptom is large and measured
(2026-05-30):

-   **193 distinct `const NAME: i32 = …`** names are redefined in **2+
    modules** each. Worst offenders: `INT_BASE` (16 copies),
    `MAX_HALFWORD` (12), `TOO_BIG_CHAR` (11), `CS_TOKEN_FLAG` (10),
    `DIMEN_BASE` (9), `TEX_NULL`/`SMALL_NODE_SIZE`/`GLUE_BASE`/
    `BOX_NODE_SIZE`/`BOX_BASE` (8 each), …
-   The duplicates currently **agree** on value, but only by luck of
    copy-paste, and grouping is already inconsistent (`65536` vs
    `65_536`; `0x1FFFFFF` vs `0x1FFF_FFF` vs `0x1FF_FFFF` for the same
    `CS_TOKEN_FLAG`). One bad edit silently diverges a module.
-   Region bases are stored as opaque literals (`INT_BASE = 7_826_729`,
    `DIMEN_BASE = 8_941_180`, `EQTB_SIZE = 8_941_458`) where the WEB
    source *computes* them (`@d int_base == dimen_base + dimen_pars`,
    etc.). The literal form hides the structure and is unverifiable by
    inspection.
-   Bare literals still appear at call sites (e.g. session 49 wrote
    `(p - 7_826_729)` inline in two `globals.rs` accessors rather than
    reference a named base).

## 2. Goal

-   One shared module — `src/consts.rs` — holding the engine's
    compile-time constants, organized to mirror tex.web's `@d` groups:
    mem layout (`MEM_TOP`, `HI_MEM_STAT_*`, node sizes, special head
    pointers), eqtb region bases (`ACTIVE_BASE` → … → `INT_BASE` →
    `DIMEN_BASE` → `EQTB_SIZE`), token/flag constants (`CS_TOKEN_FLAG`,
    `MAX_HALFWORD`, `TEX_NULL`, `NULL_FLAG`, `TOO_BIG_CHAR`), selector
    codes, command/chr codes, and the parameter counts those bases are
    built from.
-   Region bases and sizes defined as **computed chains**, not literals:
    `pub const DIMEN_BASE: i32 = INT_BASE + INT_PARS;` so the single
    source of truth is the count constants, exactly as in the WEB.
-   Every module imports from `consts` (`use crate::consts::*;` or
    explicit) instead of redeclaring. Zero per-module duplicate `const`s
    for the shared set.
-   `const _: () = assert!(DIMEN_BASE == 8_941_180);`-style static
    assertions pinning each computed base to its known good value, so the
    chain can never silently drift from the format-compatible numbers.

Non-goals: do **not** turn genuinely local, single-use, self-evident
literals (`+ 1`, `* 2`, loop bounds with obvious meaning) into named
constants — that is noise. Only literals that (a) are duplicated across
modules, (b) name a TeX-semantic quantity (a region base, node size,
flag, code), or (c) are non-obvious at the call site.

## 3. Current reality (verify before executing)

-   No `src/consts.rs` / `src/constants.rs` exists today; constants live
    as `const` items scattered per-module (often `const` inside a single
    `fn`, or at module top).
-   The values are correct and format-compatible — TRIP and the
    format-loading goldens depend on the exact region bases. **This
    workstream must not change a single value**; it only relocates and
    re-expresses them. Static assertions are the guardrail.
-   Some names collide across *different* meanings (e.g. a local
    `INT_BASE_BTL` in `input.rs`); audit names before merging — same name
    must mean the same number everywhere it is centralized.

## 4. Relationship to other workstreams

-   **02 (modules):** the shared `consts.rs` is a natural module-split
    artifact. Could be folded into 02, but it is cleaner as its own doc
    because its risk profile and sequencing differ.
-   **01 (state) / Phase C:** the *deep* literal sweep partly overlaps
    with state typing. Once `eqtb`/`mem` become typed structures with
    methods, many region-base offsets get absorbed into typed accessors
    and disappear — so a blind sweep of those *before* Phase C would be
    wasted. The cross-module duplicate centralization (Phase A below) is
    independent and safe now.
-   **04 (clippy):** after centralizing, enable `clippy::unreadable_literal`
    on `consts.rs` to enforce consistent grouping; it cannot find the
    magic numbers themselves.

## 5. Execution plan (two phases)

**Phase A — centralize duplicated cross-module consts (low-risk, do now).**
The 193 duplicated names are the pure-debt portion: same value, many
copies, no semantic change to relocate.

1.  Create `src/consts.rs`. Add it to `lib.rs` (`mod consts;`).
2.  Work in batches by `@d` group (mem layout, eqtb bases, tokens/flags,
    selectors, codes). For each group:
    a. Collect every per-module definition of each name
       (`grep -rhoE 'const NAME: i32 = [^;]+;' src/*.rs | sort -u`) and
       confirm a single value (variations in digit grouping are fine;
       *different numbers are a stop-and-audit*).
    b. Define it once in `consts.rs` — computed where it is a region
       base/size (`DIMEN_BASE = INT_BASE + INT_PARS`), literal where it
       is a genuine leaf (`MAX_HALFWORD = 0x3FFF_FFFF`). Add a
       `const _: () = assert!(NAME == <known value>);` for every computed
       base.
    c. Delete the per-module copies; add `use crate::consts::NAME;`
       (or a glob) where needed.
3.  One `@d` group per commit. `cargo check` + the full dashboard
    (29/2/7/4) per commit — relocation must be provably value-preserving.

**Phase B — call-site literal sweep (pair with Phase C / newtypes).**
Replace remaining bare literals that name a TeX quantity at the point of
use (node-size arithmetic, region-offset math like the session-49
`p - 7_826_729`, command/chr comparisons) with the `consts` names. Do
this module-by-module **as each module is threaded in Phase C**, so a
literal is only named if it survives the state-typing rework. Skip any
literal that a typed accessor will absorb.

## 6. Validation

Identical to `06-workflow.md` steps 1/6: baseline all four suites, make
the change, re-run, commit only on zero regression. Additionally:

-   The static assertions in `consts.rs` are a compile-time guard: if a
    computed chain disagrees with the pinned value, the build fails
    before any test runs.
-   Because no value changes, parity is the real test — TRIP and the
    format-loading goldens (which depend on the exact eqtb region bases)
    must stay green on every commit.

## 7. Risks

-   **Silent value change.** A typo in a computed chain that the static
    assertion does not cover would shift a region base and corrupt the
    format file. Mitigation: assert every relocated base/size against its
    known literal; rely on TRIP.
-   **Name collisions across meanings.** The same identifier used for two
    different numbers in two modules (or a near-name like `INT_BASE` vs
    `INT_BASE_BTL`). Mitigation: audit each name's value set before
    merging; never merge across differing values.
-   **Scope creep into noise.** Naming trivial local literals bloats the
    diff and reduces readability. Mitigation: the §2 non-goals — only
    duplicated / semantic / non-obvious literals.
-   **Churn vs. Phase C.** Doing the deep sweep before threading wastes
    effort on literals that disappear into typed accessors. Mitigation:
    Phase A only is decoupled; Phase B rides Phase C.
