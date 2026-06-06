# 00 — Idiomatization strategy & principles

## Where we are: stuck at "Phase A"

`plan/02-state.md` laid out three phases for migrating global state:

-   **Phase A** — translated Rust functions read/write the C globals
    through thin accessors in `globals.rs`. "They look and feel ugly —
    that's fine. This phase is a beachhead, not the end state."
-   **Phase B** — state moves into an `EngineState` struct owned by Rust,
    bridged through a thread-local so signatures don't change yet.
-   **Phase C** — the thread-local is replaced by an explicit
    `&mut EngineState` parameter. "This is where the code becomes
    idiomatic."

Stage 1 translated all TeX algorithm bodies **and stopped at Phase A.** The
beachhead became the whole map. Concretely, as of session 132:

| Symptom | Count | Where |
|---|---:|---|
| `extern static mut` globals | 384 | `src/globals.rs` `mod raw` |
| promoted Rust `pub static mut` globals | 16 | `src/*.rs` outside `globals.rs` |
| thin global accessors | 811 | `src/globals.rs` (5,569 LOC total) |
| `#[no_mangle]` exports | 576 | all modules; 179 in `scan.rs` alone |
| `unsafe fn` | 694 | crate-wide |
| raw `unsafe { ... }` blocks | 846 by text search | crate-wide; clippy currently reports 688 undocumented blocks |
| `clippy` errors | 694 | 688 = `undocumented_unsafe_blocks`; 6 = correctness lints |
| `clippy` warnings | 358 | `unused_*`, `dead_code`, `static_mut_refs`, style |
| largest module | 18,748 LOC | `scan.rs` (also `globals.rs` 5,569, `ini.rs` 4,768) |

The raw `i32`-index memory model and the `MemIdx`/`Scaled`/typed-accessor
design from `plan/03-memory-and-nodes.md` were **never built**: the
code threads bare `i32` arena indices everywhere and reads `mem[]`
through `globals::mem_word(p)` style helpers over a `static mut *mut
MemoryWord`.

### One fact that makes Stage 2 much easier than it looks

**No in-scope C algorithm code reads engine state any more.** Every TeX
algorithm body is translated; the remaining non-macOS `.c` code is
limited to prototypes, static data, the `xetex-engine-interface.c`
`setjmp` entry shim and aarch64 linker hack, and two tiny `yhash` glue
helpers. Verified 2026-05-23: no non-macos `.c` file references
`mem[`/`eqtb[`, and `mem` is already allocated by Rust code
(`src/ini.rs`, via the C allocator helper `xmalloc_array`) behind a
C-symbol `static mut` raw pointer. The current allocation is not yet
Rust-owned in the `Vec`/`Drop` sense; `tt_cleanup` still frees it
through C-symbol pointers.

`xetex-macos.c` remains out of scope. We cannot test it here, so Stage 2
must not translate it. Inventory and readiness checks should ignore
macOS-only references except to note that they are intentionally
unvalidated platform glue.

Consequence: the dual-pointer "keep C and Rust in sync" shim that
`02-state.md` worried about is **unnecessary for TeX engine state
readers**. When we move a global's storage into `EngineState`, there is
no surviving C algorithm reader to keep in sync. But the migration must
still remove or neutralize the old C symbol definitions, Rust
`#[no_mangle]` backing statics, `#[link_name]` cleanup aliases, and the
two `yhash` helpers as applicable. The work is Rust-internal at the
algorithm level, but still has linker/cleanup edges.

## The end-state target: full Phase C (decided 2026-05-23)

The user chose the most idiomatic target. "Done" means:

1.  **All engine state lives in an `EngineState` struct** with the
    sub-struct clustering from `02-state.md` (`mem`, `eqtb`,
    `save_stack`, `input`, `strings`, `fonts`, `hyph`, `shipout`,
    `scanner`, `errors`, `config`). No `static mut` globals remain
    except genuine constants and the bridge-core error channel.
2.  **State is threaded as `&mut EngineState`** (or a narrower
    `&mut Mem` / `&mut Eqtb` where a function only touches one
    cluster — see anti-pattern A.2). The Phase-B thread-local is
    deleted.
3.  **The memory arena is a Rust-owned `Vec<MemoryWord>`** (`Mem`),
    with typed `NodePtr<K>` handles over the flat storage. Checked
    classification converts nullable TeX pointers into `NodePtr<K>`
    handles; field methods then are implemented directly on `NodePtr<K>`
    parameterized by capability traits (taking `&Mem` or `&mut Mem` context)
    and return semantic types (`Scaled`, `FontId`, `StrNumber`, valid arena
    indexes, nullable TeX pointers, and node-type wrappers). Bare `i32`
    arena indices do not appear at call sites, and sentinel-bearing
    halfwords are not mislabeled as always-valid arena indices.
4.  **`unsafe` survives only at true boundaries:** the `MemoryWord`
    union bit-fiddling (auditable in one file), the remaining FFI to
    `xetex_layout` / ICU / HarfBuzz / TECkit, and the engine entry
    point. The hundreds of incidental `unsafe` blocks and most
    `unsafe fn` declarations are gone, and with them the 688
    `undocumented_unsafe_blocks` lints.
5.  **No `#[no_mangle] extern "C"`** on internal functions. Those
    attributes exist only on the genuine FFI boundary that remains after
    the setjmp design is settled (`tt_engine_xetex_main` in C and any
    minimal Rust entry/config wrappers it must call). The current
    `c_api` self-FFI calls should disappear once Rust can call Rust
    directly.
6.  **Modules reflect subsystems**, per `plan/01-architecture.md`'s
    target layout: `scan.rs` and `globals.rs` are decomposed; no file
    is a 10k-line grab bag.
7.  **`cargo clippy` is clean** at the workspace lint level
    (`undocumented_unsafe_blocks = "deny"` in the root `Cargo.toml`),
    with correctness-lint exceptions individually `#[allow]`'d and
    justified where they reflect faithful C behavior.

The four test suites still pass at the same counts. The stable safe
crate API (`TexEngine`, `TexOutcome`, `FORMAT_SERIAL`) is unchanged.
The hidden `c_api` module may be reduced to compatibility shims or
rewritten once the Rust side no longer needs to call its own
`#[no_mangle]` symbols through FFI.

## Guardrails (the same net that carried Stage 1)

-   **Behavior-preserving.** The output bytes do not change. This is the
    single most valuable property: every refactor is validated by the
    same parity harness that validated every translation. A refactor
    that changes a log byte is a bug, not an improvement. (If a clippy
    "fix" rewords an error message or reorders output, it changed
    behavior — revert and `#[allow]` instead.)
-   **Incremental + bisectable.** Same per-commit discipline as Stage 1
    (`plan/06-workflow.md` steps 1/6/7 still apply: baseline all four
    suites, refactor, re-run all four, diff, commit small).
-   **Anti-patterns still bind.** `plan/07-anti-patterns.md` was written
    for translation but every clause applies to refactoring. Especially:
    A.4 (restructure, don't `.clone()` to silence the borrow checker —
    this will come up constantly as we introduce `&mut EngineState`),
    A.5 (no `Box<dyn Node>`), A.6 (no speculative generics), A.7 (do not
    "modernize" the algorithms).
-   **Performance is preserved.** TeX is hot in the tokenizer / expansion
    / line-break loops. Typed accessors must be `#[inline]` and compile
    to the same loads/stores as the raw index. Threading `&mut
    EngineState` must not introduce bounds-check regressions in inner
    loops — spot-check generated code or benchmark if a hot path looks
    suspicious. Never trade a measurable slowdown for tidiness without
    flagging it.

## The workstream map

Stage 2 is decomposed into workstreams, each getting its own detailed
plan doc (drafted across construction sessions; status tracked in
`STATUS.md`). Numbering reflects intended *construction/decision* order,
not a strict execution gate — see sequencing below.

| # | Workstream | Plan doc | Gist |
|---|---|---|---|
| 01 | **State & memory redesign** | `01-state-and-memory.md` | `EngineState`, `MemIdx`/`Scaled` newtypes, typed node handles/views, Phase B thread-local bridge → Phase C threading. The root-cause workstream. |
| 02 | Module decomposition | `02-modules.md` *(to draft)* | Split `scan.rs`/`globals.rs`/`ini.rs` into the `01-architecture.md` target layout. Largely mechanical once state boundaries exist. |
| 03 | De-unsafe & FFI teardown | `03-deunsafe-ffi.md` *(to draft)* | Drop `#[no_mangle] extern "C"` from internal fns; convert `unsafe fn` to safe; retire `build.rs` C compilation for emptied files. Mostly *falls out* of 01. |
| 04 | Clippy & style cleanup | `04-clippy.md` *(to draft)* | The cheap wins (~358 warnings + 6 correctness `#[allow]`s) now; the 688 unsafe-block lints resolve as a byproduct of 01/03. |
| 05 | Error handling & misc | `05-errors-and-misc.md` *(to draft)* | `error()`/`confusion()` → `Result`-shaped flow where it doesn't change behavior; dead-reference-comment cleanup; `Drop`/longjmp safety. Lowest priority; partly optional. |

## Sequencing rationale

**State & memory (01) goes first** because it is the root cause. The
single biggest lint bucket — 688 `undocumented_unsafe_blocks` — exists
*only because* state is read through `unsafe` derefs of `static mut`
raw pointers. Moving a cluster into `EngineState` and accessing it
through safe `&mut` methods **deletes** those unsafe blocks rather than
documenting them. Hand-writing 688 `// SAFETY:` comments first would be
pure waste: most of the commented blocks would then be deleted by 01.

This drives an explicit anti-task:

> **Do NOT bulk-add SAFETY comments to satisfy
> `undocumented_unsafe_blocks` before the state migration.** Let those
> 688 errors stay red until 01/03 dissolve the unsafe. The crate not
> passing clippy in the interim is expected and acceptable.

Module decomposition (02) and the state work (01) are intertwined:
the `EngineState` sub-structs *are* the module boundaries, so a clean
split depends on knowing the clusters. A purely **mechanical** pre-split
of `scan.rs` (moving function groups into submodules without changing
signatures) is optional and could precede 01 to make navigation easier
— but it is not required and must not change behavior. Decide this when
drafting `02-modules.md`.

De-unsafe/FFI teardown (03) and clippy (04) are largely *consequences*
of 01, not independent efforts. The cheap clippy wins in 04 (style
warnings, the handful of correctness `#[allow]`s) can be done at any
time as low-risk filler, and are a good way to spend the tail of a
session. The 688-block bucket waits for 01/03.

Error-handling (05) is the most invasive-for-least-parity-safety and is
deliberately last and partly optional.

**Typing rides with threading, not after it (decided 2026-06-06).**
Targets #3 (typed `Mem`/`NodePtr<K>`) and #4 (de-unsafe) are *not* a
separate call-site sweep that follows Phase C. The early Phase-C slices
threaded with bare `i32` for speed, but every *remaining* slice now
carries its typing in the same touch (`01`-§4 option (c), `01b` §3a):
when a function gains its `&mut`-state signature it also adopts
`NodePtr<K>`/`MemIdx`/`MemPtr`/`Scaled` and sheds its `unsafe`. This
avoids a second full pass over the same functions; the only prerequisite
is building the typed foundation up front (scalar newtypes, the `Mem`
ownership move, the per-family `NodePtr<K>` layer — `01`-§4a, executed per
the `01c` cookbook), and the only residual backlog is the
deferred-newtype worklist, not a sweep.

## A hazard to design around now: longjmp vs. `Drop`

The engine entry point `tt_engine_xetex_main` stays in C precisely
because it owns a `setjmp`/`longjmp` error escape (`plan/MODULE_STATUS.md`
Phase 6). `longjmp` unwinds the C stack **without running Rust
destructors.** Today that is harmless because engine state is in
`static mut` globals with no `Drop`. Once `EngineState` owns `Vec`s and
other `Drop` types and lives on the Rust stack across the `longjmp`
boundary, a `longjmp` would leak (or worse). The Phase-B thread-local
form sidesteps this (state lives in a `'static` thread-local, not a
stack frame that gets jumped over), which is an additional reason to go
through Phase B rather than jumping straight to stack-threaded Phase C.
The `01` plan must treat "where does `EngineState` live relative to the
longjmp" as a first-class design question, not an afterthought.

The companion hazard is cleanup ownership: current `tt_cleanup` frees
large arrays through `#[link_name]` aliases to C symbols (`mem`, `eqtb`,
`str_pool`, font arrays, trie arrays, etc.). As clusters move into
`EngineState`, `tt_cleanup` must stop `free()`ing storage that is now
owned by a `Vec`/Rust wrapper, and `EngineState` must provide explicit
reset/teardown for both normal return and abort paths.

## What success looks like, measured

The same dashboard discipline as Stage 1, plus idiom metrics tracked in
`STATUS.md`: count of C-symbol globals, promoted Rust statics,
`unsafe fn` items, raw unsafe blocks, `#[no_mangle]` exports, clippy
errors/warnings, and largest-module LOC — all trending to their targets
(0, 0, ~boundary-only, ~boundary-only, boundary-only, 0, and
"no >~3k-line module") while the four test suites never move off
29/2/7/4.
