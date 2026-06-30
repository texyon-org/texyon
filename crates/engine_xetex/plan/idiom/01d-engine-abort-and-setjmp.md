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

**Behavior-preservation invariant.** The outer Rust caller
(`lib.rs:267`) maps the C return purely by value: `0/1/2 → Ok(...)`,
`3 → Err(EngineAbortedError::new_abort_indicator())`. The `_tt_abort` *message*
(`format_buf`) is **not** surfaced — user-facing diagnostics come entirely from
the log printing done *before* the jump (`pre_/post_error_message`). Therefore
**returning `HISTORY_FATAL_ERROR` (3) from `tt_run_engine` is observationally
identical to longjmp-ing**, provided the same printing + cleanup ran first. This
is the entire correctness basis for the conversion.

## 2. The corrected lever — the bridge_core I/O tail (key finding)

Converting the whole Rust `_tt_abort` spine to `Result` is **necessary but not
sufficient** to delete the setjmp. Sources (2) and (3) — `ttstub_input_seek` /
`ttstub_input_close` — `longjmp` *from C*, on every input seek/close the engine
performs (called from `io.rs`, `format_io.rs`, `texmfmp.rs`, `fonts.rs`,
`pic.rs`, `ext.rs`; including inside `close_files_and_terminate` at `io.rs:921`
and the format-undump close at `format_io.rs:1476` — i.e. inside the boundary
functions themselves). A boundary fn holding `&mut EngineState` across one of
those FFI calls is still UB on the (rare) internal-error jump.

So the honest end-state is staged:

- **Stage I (this track):** convert the *Rust* fatal spine to `Result`/`EngineAbort`
  so the only longjmps left are the two bridge_core I/O ones. The setjmp stays as
  a thin I/O-only backstop. This is fully inside `engine_xetex`.
- **Stage II (cross-crate, gates the final flip):** make `ttstub_input_seek`/
  `ttstub_input_close` return error codes that Rust checks and propagates, instead
  of `longjmp`-ing. `bridge_core` is shared with bibtex/dvipdfmx/xdvipdfmx, so
  this has blast radius beyond this crate and must be coordinated. Only after
  Stage II can the setjmp be deleted and `tt_run_engine` own `EngineState`
  outright.

This caveat does **not** reduce the value of Stage I: it removes the scattered
deferred-`fire` longjmp machinery, unifies it, and makes the boundary functions
abort-safe w.r.t. *fatal* aborts — the dominant case. It just means "delete the
setjmp" is gated on a bridge_core change the pivot framing omitted.

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
5. **`main_control` cluster** — only after the Phase-C `with_state` drain lets
   `main_control` carry a borrow / return `Result`. Folds `MainControlAbort`,
   `PageBuildAbort`, `SccAbort`, the linebreak/output/save/input aborts.

Each step: fold the leaf `*Abort`(s) it owns into `EngineAbort`; keep the setjmp
backstop so unconverted callers still abort identically; dashboard green per
commit.

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
