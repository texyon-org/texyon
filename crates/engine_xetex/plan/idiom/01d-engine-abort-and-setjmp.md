# EngineAbort: killing the Rust longjmp spine

The architectural lever behind the last `with_state` wrappers and the `mod raw`
scalars. Companion to [`01b-phase-c-recipe.md` §5](01b-phase-c-recipe.md) (the
per-subsystem `*Abort` sentinel) and the memory note
`globals-wrappers-gated-on-setjmp-boundary`.

**Thesis (session 241 pivot).** The remaining ~98 `globals.rs` `with_state`
wrappers and the 41 `mod raw` scalars do **not** drain by folding more boundary
callers — for singletons, `set_X(v)` → `with_state_mut(|s| s.f = v)` just
*relocates* the borrow. They drain only when the setjmp/IO boundary functions
(`tt_run_engine`, `store_fmt_file`, `final_cleanup`, …) can hold a real
long-lived `&mut EngineState`. They cannot today because a `longjmp` over their
frame would discard the borrow guard without running `Drop` (panic-on-next-access
under the `thread_local! RefCell`; leak/UB under owned state). **The lever is
killing the Rust longjmp, not folding wrappers.**

---

## 1. Verified architecture

`tt_engine_xetex_main` (`xetex/xetex-engine-interface.c:33`):

```c
if (setjmp(*ttbc_global_engine_enter(api))) {   // line 33
    ttbc_global_engine_exit();
    return HISTORY_FATAL_ERROR;                  // 3
}
rv = tt_run_engine(dump_name, input_file_name, build_date);
ttbc_global_engine_exit();
return rv;
```

Verified:

- **One-shot fatal bailout, no retry, no signals.** The only `setjmp` is line 33;
  the `__terrible_aarch64_musl_linker_hack_never_call_me` fn (line 76) is a
  never-called linker hack with its own throwaway buffers. Nothing
  signal/interrupt-driven depends on the jmp_buf.
- **Three longjmp sources**, all → that one setjmp (`bridge_core/support/support.c`):
  1. `_tt_abort(fmt)` (line 102) — explicit fatal abort, called by the Rust
     fatal spine.
  2. `ttstub_input_seek` (line 282) — `longjmp` on a *serious internal* seek error.
  3. `ttstub_input_close` (line 320) — `longjmp` on a *serious internal* close error.
- **Recoverable `error()` already returns normally.** Only the fatal set
  (`overflow`/`confusion`/`fatal_error`/`pdf_error`/direct `_tt_abort`) jumps;
  `error_in` returns `ErrorOutcome` and only its `Halt`/`Hundred` arms abort.

**Behavior-preservation invariant.** The outer Rust caller (`lib.rs:267`) maps
the C return by value: `0/1/2 → Ok(...)`, `3 →
Err(EngineAbortedError::new_abort_indicator())`. **Caveat (corrects an earlier
draft):** the abort message string *is* surfaced — `bridge_core`'s
`with_global_lock` (`lib.rs:320`) detects the `EngineAbortedError` and replaces
it with `new_with_details()`, which reads the C `format_buf` via
`_ttbc_get_error_message()`. `_tt_abort(msg)` writes `format_buf`; the
`Result`-return path bypasses `_tt_abort`, so it **must** write the same string
itself via `_ttbc_set_error_message(msg)` (added to `bridge_core/support.c`,
called from `EngineAbort::into_history` — `"halted on overflow()"` /
`"halted on confusion()"` / the `TtAbort` `msg`). With that,
**returning `HISTORY_FATAL_ERROR` (3) from `tt_run_engine` is observationally
identical to longjmp-ing** — same log printing, same cleanup, same C return,
same surfaced message. This is the correctness basis for the conversion. (The
log/terminal diagnostics from `pre_/post_error_message` are unchanged regardless;
the surfaced-string fidelity is the part that needs `_ttbc_set_error_message`.)

## 2. The bridge_core I/O tail — and why it's IN-CRATE (corrected)

Converting the whole Rust `_tt_abort` spine to `Result` is **necessary but not
sufficient** to delete the setjmp. Sources (2) and (3) — `ttstub_input_seek` /
`ttstub_input_close` — `longjmp` *from C*, on input seek/close the engine
performs (called from `io.rs`, `format_io.rs`, `texmfmp.rs`, `fonts.rs`,
`pic.rs`, `ext.rs`; incl. inside `close_files_and_terminate` and the
format-undump close — i.e. inside the boundary functions). A boundary fn holding
`&mut EngineState` across one of those FFI calls is UB on the (rare)
internal-error jump.

**But this is NOT a cross-crate change (corrects an earlier draft).** The longjmp
lives *only* in the C `ttstub_input_seek`/`ttstub_input_close` wrappers
(`bridge_core/support/support.c:280-283, 318-321`). Those wrappers call the Rust
`ttbc_input_seek` (`bridge_core/src/lib.rs:1107`) / `ttbc_input_close` (`:1233`),
which **return the error as a value and never jump** — they wrap
`CoreBridgeState::input_seek -> Result<u64>` / `input_close -> bool`. So
`engine_xetex` kills the I/O longjmp entirely on its own side: **stop calling the
C `ttstub_*` seek/close wrappers; call `ttbc_input_seek`/`ttbc_input_close`
directly** (both `#[no_mangle] pub extern "C"`, already exported + linked —
support.c calls them the same way), or **thread `&mut CoreBridgeState` from
`tt_engine_xetex_main` into `tt_run_engine` and call the Rust methods directly**
(they already return `Result`). The shared C `ttstub_*` wrappers stay untouched
for bibtex/dvipdfmx — **no other-engine impact, no coordination.**

So the end-state is staged, but **both stages are engine_xetex-local**:

- **Stage I (this track):** convert the *Rust* `_tt_abort` spine to
  `Result`/`EngineAbort` up to `tt_run_engine`. Setjmp stays as a thin backstop.
- **Stage II (also in-crate):** replace the ~19 `ttstub_input_seek/close` call
  sites with `ttbc_*`/`CoreBridgeState` + error→`Result` threading (same
  mechanical work as Stage I, just for the I/O error paths). Cleanest is the
  `CoreBridgeState::input_seek` `Result` (the C `ttbc_input_seek` out-param has a
  `TODO` and doesn't set `internal_error` on a real seek `Err`). **Also fold the
  allocator jumps:** `_tt_abort` is called from `xmalloc/xcalloc/xrealloc`
  (support.c) and `tt_run_engine` uses `xmalloc` — replace those few with Rust
  allocation (or accept, since OOM is terminal). After Stage II nothing longjmps,
  and the setjmp + owned-`EngineState` flip (§6) becomes possible.

Stage I already removes the scattered deferred-`fire` machinery, unifies it, and
makes the boundary functions abort-safe w.r.t. *fatal* aborts — the dominant
case. Stage II is not a blast-radius wildcard; it is a bounded in-crate sweep.

## 3. Current state — already deferred-fire

The Rust fatal spine is **already** almost entirely deferred-fire: threaded `_in`
cores return a per-subsystem `*Abort` sentinel and the thin global wrapper
`fire()`s it *after* the borrow drops. The `overflow`/`confusion`/… calls sit
**inside** the `fire` impls, not under a live borrow. So the threaded cores are
safe; what longjmps is the wrapper's `fire()`.

37 `*Abort` enums + 11 `fire`/`fire_*` fns exist. **Every one reduces to the same
five terminal causes** (verified by reading all `fire` bodies):

| terminal | prints | then |
|---|---|---|
| `overflow(what, n)` | "TeX capacity exceeded…" | `post_error_message(true)` + `_tt_abort` |
| `confusion(s)` | "This can't happen (s)" | `post_error_message(true)` + `_tt_abort` |
| `fatal_error(s)` | "Emergency stop" + s | cleanup + `_tt_abort` |
| `pdf_error(t, p)` | "Error (t): p" | `post_error_message(true)` + `_tt_abort` |
| `_tt_abort(msg)` | *nothing user-facing* | longjmp (msg → `format_buf`, unsurfaced) |

`ErrorOutcome` (`Halt`/`Hundred` = `post_error_message(false)` + `_tt_abort`) and
`LineBreakOutcome` are sub-routers to these. `fire_main_memory_overflow` →
`overflow`; `fire_name_overflow` → `overflow`; `fire_make_tex_string` →
`overflow`/`_tt_abort`. Nested aborts (`PageBuildAbort::Input(InputAbort)`, …)
delegate to the inner `fire`.

## 4. `EngineAbort` — the unified terminal type

```rust
// errors.rs
pub(crate) enum EngineAbort {
    Overflow { what: *const c_char, n: i32 },
    Confusion(*const c_char),
    Fatal(*const c_char),
    PdfError { t: *const c_char, p: *const c_char },
    TtAbort(*const c_char),
    /// post_error_message(false) then _tt_abort(msg) — ErrorOutcome Halt/Hundred.
    HaltMessage(*const c_char),
}
```

Two consumers, the same six arms:

- `unsafe fn fire(self) -> !` — the **single** longjmp dispatcher (calls the
  existing `overflow`/`confusion`/… which print+cleanup+`_tt_abort`). Used by the
  still-unconverted leaf wrappers + the bridge backstop.
- `unsafe fn into_history(self) -> i32` — prints+cleans **without** the trailing
  `_tt_abort`, returns `HISTORY_FATAL_ERROR`. Used at every `Result` catch point.
  Requires splitting `overflow`/`confusion`/`fatal_error`/`pdf_error` into
  `*_message()` (print+cleanup, returns `()`) + the `-> !` longjmp wrapper
  (`= { *_message(); _tt_abort(...) }`).

`into_history(TtAbort)` returns 3 with no cleanup — matching `_tt_abort`'s
longjmp, which also does no cleanup (cf. the existing `return history()` early
exits in `tt_run_engine`).

Leaf `*Abort` enums fold into `EngineAbort` incrementally: each gains
`fn into_engine_abort(self) -> EngineAbort` and its `fire()` becomes
`self.into_engine_abort().fire()`. **Do not big-bang all 17 files** — fold a leaf
only when its subsystem converts to propagate `Result<_, EngineAbort>`, so each
commit stays a real lever (deleted longjmps), not plumbing cosmetics.

## 5. Conversion order — top-down from `tt_run_engine`

The catch point is `tt_run_engine`, which returns `i32` → C return. Subsystems
reachable **without** going through `main_control` convert first (they propagate
straight to `tt_run_engine`); `main_control`-reached leaves wait on the Phase-C
`with_state` drain (see `main-control-single-borrow-premature`) and keep
longjmping via the setjmp backstop until then.

1. **`tt_run_engine` init fatals** (in-frame, trivial): consistency check
   `_tt_abort` (`engine.rs:1199`), `fire_primitive_init_abort` (1209) →
   `return EngineAbort::_.into_history()`.
2. **`final_cleanup`** → `Result<(), EngineAbort>`, caught at `tt_run_engine`
   (deletes the `confusion` at `engine.rs:420`).
3. **`store_fmt_file`** → `Result<(), EngineAbort>` (deletes `_tt_abort`/`overflow`
   at `format_io.rs:217/238/245/256`). Requires threading `dump_bytes`
   (`format_io.rs:103`) and the session-241 dump-region closures to return
   `Result` so `?` propagates. Larger; its own slice.
4. **`load_fmt_file`** → `Result`, propagate the `!load_fmt_file()` early-return
   at `tt_run_engine:1275`.
5. **`main_control` cluster** — **UNBLOCKED** (session 243 disproved the stale
   gate). See §5.5.

Each step: fold the leaf `*Abort`(s) it owns into `EngineAbort`; keep the setjmp
backstop so unconverted callers still abort identically; dashboard green per
commit.

## 5.5 The `main_control` fold (next session — fully scoped)

`main_control()` is already `match with_state_mut(|st| main_control_in(st)) {
Ok(()) => {} Err(e) => e.fire() }`, with `main_control_in` holding **one** borrow
(no `with_state` leaf below it — verified). The `e.fire()` is **post-borrow**, so
this is critical-path progress toward "tt_run_engine catches everything", not a
borrow-safety fix. Target:

```rust
pub unsafe fn main_control() -> Result<(), crate::errors::EngineAbort> {
    crate::state::with_state_mut(|st| main_control_in(st)).map_err(|e| e.into_engine_abort())
}
// tt_run_engine: if let Err(a) = main_control() { return a.into_history(); }  // skips final_cleanup, == old longjmp
```

All 6 `EngineAbort` terminals now exist (Overflow/Confusion/Fatal/PdfError/
TtAbort/HaltMessage), each with an `into_history` arm — the foundation is
complete. Remaining work is `MainControlAbort::into_engine_abort()` mirroring its
`fire()`. **Leaf-terminal map** (transcribe each leaf's `fire()`/`finish()` into
an `into_engine_abort`):

| leaf | terminal → EngineAbort |
|---|---|
| `overflow(w,n)` / `fire_main_memory_overflow` / `*::MainMemory` | `Overflow{w,n}` (main mem = `{"main memory size", MEM_TOP+1}`) |
| `confusion(s)` / `*::Confusion(s)` | `Confusion(s)` |
| `fatal_error(s)` | `Fatal(s)` |
| `pdf_error(t,p)` | `PdfError{t,p}` |
| `_tt_abort(msg)` | `TtAbort(msg)` |
| `ErrorOutcome` (Halt/Hundred/Confusion/Overflow) | `HaltMessage`/`Confusion`/`Overflow` |
| `TrieAbort` | `Overflow{"pattern memory[…]", size}` |
| `MakeTexStringAbort` (PoolOverflow/StringOverflow) | `TtAbort("string pool overflow")` / `Overflow{"number of strings",…}` |
| `NameAbort`/`OpenLogAbort`/`OpenOrCloseAbort` | recurse to inner (Name→Overflow, Scanner→Scc, MakeString→MakeTexString) |
| `SaveAbort`/`SaMutationAbort`/`UnsaveAbort` | Overflow/Confusion (save) |
| `PageBuildAbort`/`ShipoutAbort`/`InsertSrcAbort`/`Issue/ShowWhatever`/`ReadFontInfo`/`NewCharacter`/`PushNest`/`Vsplit`/`Input`/`NodeRecycle` | recurse to inner / their single terminal |

**Tricky (NOT pure value maps — `into_engine_abort` is `unsafe` + side-effecting
for these):**
- `SccAbort::NoLegalEnd{open_log}` — opens the log (if `!log_opened`) **then**
  `Fatal("*** (job aborted, no legal \\end found)")`. Keep the open_log side
  effect before returning `Fatal`.
- `scan::node_recycle_abort::OverflowRunaway` — calls `runaway()` **then**
  `Overflow{"main memory size",…}`.
- `ErrorOutcome`'s `post_error_message` runs inside `into_history(HaltMessage)`
  (correct timing — post-borrow, at the catch), not in `into_engine_abort`.

**Validation:** all-or-nothing (one commit; leaf maps are dead until
`main_control` uses them). Dashboard exercises only recovery, not fatal aborts —
so **adversarially verify** each leaf map against its `fire()` (the session-242
verify caught the `format_buf` surfacing; the same risk class applies per leaf).
Likely **decreases** `unsafe fn` (delete `MainControlAbort::fire` + any
now-unused `fire_*` whose sole caller was main_control). Corpus at the end.

## 5.6 The Stage-I residue census (session 246 — the non-`main_control` scatter)

After the `main_control` fold, the remaining Rust `_tt_abort` spine was the
**non-`main_control` scatter**: ~15 global wrappers + a handful of inline
longjmps, each varying in reachability (unlike `main_control`, which was one call
from `tt_run_engine`). This session **converted the whole directly-propagatable
`(A)` set (8 sites, verified 8/8 EQUIVALENT by a 7-agent adversarial workflow +
transitive coverage of the 8th)** and **censused the rest**. Reachability is now
pure Rust caller analysis: the C core is fully ported (`xetex-xetex0.c` is a
10-line empty stub; the ~10 remaining `xetex/*.c` files are 10–101-line helper
stubs that never call *back* into these Rust wrappers), and every wrapper is
plain `pub unsafe fn` (Rust-ABI — C cannot call it), so a fire-site blocks the
setjmp **only if a live Rust path reaches it**.

**Converted this session — `(A)`, reachable on a Result-threadable path to
`tt_run_engine` (each verified EQUIVALENT to its old `fire()`/dispatcher):**

| wrapper / arm | leaf `into_engine_abort` | caught at |
|---|---|---|
| `start_input` | `SccAbort` | `tt_run_engine` direct (`into_history`) |
| `maketexstring` | `MakeTexStringAbort` | `tt_run_engine` INITEX init (direct, `into_history`) |
| `get_strings_started` node_recycle arm | `NodeRecycleAbort` | `tt_run_engine` direct |
| `store_fmt_file` node_recycle arm | `NodeRecycleAbort` | `final_cleanup` `?` |
| `pack_job_name` | `MakeTexStringAbort` | `store_fmt_file` `?` → `final_cleanup` |
| `make_name_string` | `NameAbort` | `store_fmt_file` `?` → `final_cleanup` |
| `init_trie` | `TrieAbort` | `store_fmt_file` trie-dump `?` → `final_cleanup` |
| `end_token_list` | `InputAbort` | `final_cleanup` `?` |

(Deleted the now-dead `trie_abort` dispatcher. `unsafe fn` 263, warnings 37.)

**DEAD — no live caller (Rust or C); do NOT block setjmp deletion (deletable now
or at the flip):** `input_line`, `open_or_close`, `more_name`, `end_name`,
`scan_and_pack_name`, `scan_file_name`, `scan_file_name_braced` (`io.rs`),
`make_src_special` (`texmfmp.rs`), `set_input_file_encoding` (`io.rs`, sole caller
`u_open_in` is itself dead). Each was superseded by its threaded `_in` core
(`input_line_in`, `u_open_in_in`, …); the `xetex-xetexd.h` C prototypes are
vestigial (declare C-ABI symbols no live C references). *Verified: 0 call-sites.*

**`(A)` but non-trivial — convertible, NOT a wrapper-flip, deferred:** `error()`
(`errors.rs`). The audit **corrected the recon**: `error()` has **no genuinely
un-threadable caller** — its 4 firing sites are `store_fmt_file` ×2 (already
`Result`), `post_error_message` (inside `into_history`'s terminal machinery — the
`Halt`/`Hundred` outcome is absorbable, cleanup already commits to
`HISTORY_FATAL_ERROR`), and `m_log`'s `x ≤ 0` branch (a **dead** branch —
`norm_rand`/`\normaldeviate` always feeds `x ≥ 1`; and it runs under
`conv_toks_in`'s held borrow with a `Result` path, so it would double-borrow-panic,
not longjmp). So `error()` does **not** fundamentally block the flip — but
converting it is a multi-site `ErrorOutcome`-threading change touching the
recoverable-error machinery, so it is a separate follow-up (needs its own verify).

**`(B)` flip-gated — deferred-fire wrapper with an un-threadable
terminal-machinery caller (the genuine deferred blockers):**

| wrapper | threadable caller | blocking caller (un-threadable) |
|---|---|---|
| `open_log_file` (`io.rs`) | `final_cleanup` | `pre_error_message` inside `into_history`'s terminal print path (returns `i32`, IS the catch point) |
| `finalize_dvi_file` (`shipout.rs`) | `tt_run_engine` normal exit | `close_files_and_terminate`, reached from 3 terminal abort-cleanup sites inside `into_history` (`post_error_message`, the `Fatal` arm, `store_fmt_file` `\dump`-in-group) |

Converting either forces its shared cleanup helper (`pre_error_message` /
`close_files_and_terminate`) to return `Result`, gated on how the terminal
`into_history` handlers absorb a *second* error during cleanup — a §6-flip concern.

**Inline longjmps — direct `_tt_abort`/`fatal_error`, not deferred-fire; block the
setjmp until restructured:**
- `main_control.rs:706` (`get_preamble_token_in`): direct
  `fatal_error("(interwoven alignment preambles are not allowed)")` **under the
  `&mut EngineState` borrow** — a still-live longjmp reachable from
  `main_control`'s alignment path, distinct from the folded deferred
  `MainControlAbort` spine. Needs threading as a `MainControlAbort`/align variant.
- `ext.rs` native-font helpers (**13** sites): direct `_tt_abort` — 11×
  `"bad native font flag in <fn>"` (`measure_native_node`, `measure_native_glyph`,
  `get_native_char_height_depth`, `get_native_char_side_bearings`,
  `get_glyph_bounds`, `getnativecharwd`, `map_char_to_glyph`, `map_glyph_to_index`,
  `get_font_char_range`, `make_font_def`, `print_glyph_name`), `"could not read
  mapping file"` (`load_mapping_file_in`), `"failed to create linebreak iterator"`
  (`linebreak_start_in`). All return `()`/`i32` (no `Err` channel) and are called
  **deep** from packaging/linebreak/math/scan/expand measurement — internal-
  consistency + resource aborts. These thread only when the native-font measurement
  layer grows an error channel.

**Blocker summary (what still gates setjmp deletion, post-session):** the Rust
longjmps are now a **small, enumerable set** — the two `(B)` wrappers
(`open_log_file`, `finalize_dvi_file`) + `error()` (`(A)`-nontrivial) + the two
inline classes (`main_control` interwoven-align, `ext.rs` native-font ×13), plus
the dispatcher machinery reachable *only through* those live wrappers
(`OpenLogAbort::fire` under `open_log_file`; `fire_shipout_abort`/`SccAbort::fire`
under `finalize_dvi_file`). Everything else reachable-and-threadable is now
`Result`; the DEAD set never fires. The `(A)`/`(B)` split came out **`(A)`-heavier
than expected** (8 converted + `error()` convertible) and **`(B)`-light** (only 2
true deferred blockers), with the structural remainder being the two inline
classes. This is the precondition proof for **Stage II** (§2): neutralize the
C-side `bridge_core` I/O longjmps (`ttstub_input_seek`/`close`) + `xmalloc`; then
the flip (§6) is gated only on `error()` + the two inline classes threading.

## 6. Final flip (all-or-nothing, later session)

Once **no Rust path longjmps** (steps 1–5 done) *and* Stage II (§2) neutralizes
the bridge_core I/O longjmps:

- `tt_run_engine` owns `let mut st = EngineState::new();` and threads `&mut st`
  everywhere; its body becomes `match inner(&mut st) { Ok(h) => h, Err(a) =>
  a.into_history() }`.
- `xetex-engine-interface.c` drops the `setjmp`; `tt_run_engine`'s `i32` return is
  the sole channel.
- `with_state`/`with_state_mut`, the `thread_local! RefCell`, the ~98 `globals.rs`
  wrappers, and the 41 `mod raw` scalars all delete **together** (one commit) —
  there is no live `with_state` leaf left to nest-panic under the owned `&mut`.

This is gated, not scheduled: it lands only when both preconditions hold.

## 7. Session 242 slice (this session)

Stage-I bootstrap, setjmp kept as backstop:

1. **Commit 1.** `EngineAbort` (variants used: `Overflow`, `TtAbort`) +
   `*_message` splits + `into_history` in `errors.rs`; convert the two
   `tt_run_engine` init longjmps (steps §5.1) to `into_history`. −2 Rust longjmps.
2. **Commit 2.** `final_cleanup` → `Result<(), EngineAbort>` caught at
   `tt_run_engine` (adds `Confusion`); fold its `cond_ptr` multi-read loop. −1.

Validation each commit: `cargo fmt`/`check` (warnings 37), dashboard 29/2/7/4
(TRIP covers `\dump`/undump + `\if`-recovery — the high-risk fatal/cleanup
paths), release rebuild, full corpus 637/2 at the end. `unsafe fn` must not rise
from 273. Verify a converted fatal path still terminates with the same
history/exit (3 → `EngineAbortedError`), not silently continue.
