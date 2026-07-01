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
- **Stage II (also in-crate) — 15 of 17 seek/close sites done (session 247, §2.1).**
  Replace the `ttstub_input_seek/close` call sites with the exported `ttbc_*` C-ABI
  fns + error→`Result` threading (same mechanical work as Stage I, for the I/O error
  paths). **Decision was Option A, not `CoreBridgeState::input_seek`'s `Result`:** that
  method is a **private `fn`** in bridge_core, so it isn't callable in-crate — only the
  exported `ttbc_input_seek`/`ttbc_input_close` are (§2.1). All 7 seeks + 8 of 10
  closes converted (11/11 adversarial-verified EQUIVALENT); **2 closes deferred**
  (`u_close` (B)-class terminal-cleanup caller; `load_mapping_file_in` folds with the
  ext native-font inline class, §5.6). **The `xmalloc/xcalloc/xrealloc` `_tt_abort`
  jumps stay deferred** (OOM is terminal — accept per §2). So after session 247 the
  bridge_core input-I/O longjmp is gone from every reachable Result-threadable path;
  the two deferred closes + the Rust `_tt_abort` residue (§5.6) remain before the
  setjmp + owned-`EngineState` flip (§6) becomes possible.

Stage I already removes the scattered deferred-`fire` machinery, unifies it, and
makes the boundary functions abort-safe w.r.t. *fatal* aborts — the dominant
case. Stage II is not a blast-radius wildcard; it is a bounded in-crate sweep.

## 2.1 Stage II — the A-vs-B decision (session 247: **Option A**)

**Recon settled it: Option A — call the exported C-ABI `ttbc_input_seek` /
`ttbc_input_close` directly, obtaining `&mut CoreBridgeState` from the existing
`_ttbc_get_core_state()` accessor.** Not B (thread `&mut CoreBridgeState` from
`tt_engine_xetex_main` through `tt_run_engine` and the deep I/O tree).

Why A, decisively:

1. **B is not even in-crate for seek.** `CoreBridgeState::input_seek`
   (`bridge_core/src/lib.rs:684`) is a **private `fn`** (no `pub`) — engine_xetex
   cannot call it without a `bridge_core` edit, contradicting "in-crate." Only the
   exported `#[no_mangle] pub extern "C" fn ttbc_input_seek` (`:1107`) is reachable.
   (`input_close` at `:711` *is* `pub`, but that lone method doesn't justify the B
   plumbing.) So the exported `ttbc_*` C-ABI symbols are the only in-crate path — A.
2. **Both A and B need `&mut CoreBridgeState`.** A gets it for free from
   `_ttbc_get_core_state()` (`bridge_core/support/support.c:128`, plain exported
   fn returning the `tectonic_global_bridge_core` global the C `ttstub_*` wrappers
   already use). engine_xetex already declares such non-header externs
   (`_ttbc_set_error_message`, `errors.rs:51`). B would instead thread a **second
   long-lived `&mut` state param** from the C entry through `tt_run_engine` and
   every deep I/O call site (`io`/`texmfmp`/`format_io`/`pic`/`fonts`/`ext`) —
   alongside the `thread_local! RefCell` `EngineState` borrow — a large blast
   radius that fights the with_state architecture and buys nothing over A.
3. **The longjmp lives only in the C `ttstub_*` wrappers.** `ttbc_input_seek` /
   `ttbc_input_close` return the error as a value and never jump; calling them
   directly neutralizes the longjmp at the source (the setjmp precondition).

**Plumbing sketch (io.rs — shared, `pub(crate)`):**

```rust
extern "C" {                       // non-header exports (like _ttbc_set_error_message)
    fn _ttbc_get_core_state() -> *mut libc::c_void;   // the global CoreBridgeState
    fn ttbc_input_seek(es: *mut libc::c_void, handle: usize, offset: libc::ssize_t,
                       whence: libc::c_int, internal_error: *mut libc::c_int) -> libc::size_t;
    fn ttbc_input_close(es: *mut libc::c_void, handle: usize) -> libc::c_int;
}
/// Non-longjmp seek. Every engine site passes SEEK_SET, and ttbc_input_seek sets
/// internal_error only for an invalid whence, so the seek longjmp is UNREACHABLE:
/// returns the new position like the C ttstub_input_seek wrapper, asserts ie==0.
pub(crate) unsafe fn input_seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int) -> libc::size_t { … }
/// Non-longjmp close. Returns true on the serious internal close error the C
/// ttstub_input_close wrapper `longjmp`'d on (ttbc_input_close != 0); Result-path
/// callers propagate it as EngineAbort::TtAbort (no cleanup, sets the surfaced
/// message, history 3) — mirroring the old bare longjmp's control flow.
#[must_use] pub(crate) unsafe fn input_close(handle: usize) -> bool { ttbc_input_close(_ttbc_get_core_state(), handle) != 0 }
```

**Behavior-preservation.** *Seek* — provably no abort (SEEK_SET ⟹ `internal_error`
never set; the C `ttbc_input_seek` out-param's TODO — misreporting a *real* seek
`Err` as position 0 with no `internal_error` — is moot here because the error arm
is unreachable at every rewind site; A inherits the exact unreachable behavior).
*Close* — the old bare `longjmp(jump_buffer,1)` (support.c:332, **not** `_tt_abort`)
ran no cleanup and left `format_buf` undefined; `EngineAbort::TtAbort(msg)`
(`errors.rs`) into_history runs no cleanup, sets a **descriptive** message, and
returns 3 — control-flow identical, the message a documented, test-invisible
improvement over "undefined stale `format_buf`".

**`CoreBridgeState` aliasing is sound.** `_ttbc_get_core_state()` returns the same
non-null global the C `ttstub_*` wrappers pass; engine_xetex holds no other
reference to it, so forming a fresh `&mut` per ttbc call mirrors the C side exactly.

**Reachability of the close abort → tractability (session-247 census).** A close
error longjmps only when `input_close`'s `scan_remainder()` fails (rare mid-read
I/O error). Faithful preservation needs each close site's enclosing fn to
propagate that as a `Result`. Live-caller analysis:

| close site (fn) | live caller → error type | this session |
|---|---|---|
| `load_fmt_file_in` (format_io) | self — `Result<_, EngineAbort>` | convert (direct `TtAbort`) |
| `read_font_info_in` (fonts) | self — `Result<_, ReadFontInfoAbort>` | convert (+ `InputClose` variant) |
| `getfilesize_in`/`getfiledump_in`/`getfilemoddate_in` (texmfmp) | `conv_toks_in` — `Result<_, SccAbort>` (the non-`_in` wrappers are DEAD) | convert (+ `SccAbort::InputClose`) |
| `find_pic_file` (pic) | `load_picture_in` — `Result<_, MainControlAbort>` | convert (+ `MainControlAbort::InputClose`) |
| `count_pdf_file_pages_named` (pic) | `scan_something_internal_in` — `Result<_, SccAbort>` | convert (`SccAbort::InputClose`) |
| `u_close` (io) | `open_or_close_in` (Result) **and** `close_files_and_terminate` (terminal cleanup, un-threadable) | **DEFER — (B)-class**: its abort is reached from `into_history`'s terminal machinery, a §6-flip concern |
| `load_mapping_file_in` (ext) | itself already has an inline `_tt_abort` ("could not read mapping file") | **DEFER** with the ext.rs native-font ×13 inline-longjmp class (§5.6) |

Seek sites: `u_open_in` (DEAD — delete, superseded by `u_open_in_in`; drops 3 sites
+ its sole callee `set_input_file_encoding`), `u_open_in_in` (×3, Result but no
abort), `getfiledump_in` (×1). So this session converts **15 of 17** sites and
defers **2** (`u_close`, `load_mapping_file_in`) with the (B)/native-font rationale
above — the `xmalloc` fold stays deferred (OOM is terminal, §2).

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

**`(A)` but non-trivial — DONE (session 248):** `error()` (`errors.rs`) deleted;
all 4 callers threaded, 6/6 adversarial-verified EQUIVALENT. The recon-corrected
census held: `store_fmt_file` ×2 (already `Result`) → `error_in` +
`Err(into_engine_abort())` on non-`Continue`; `post_error_message` (inside
`into_history`'s terminal machinery) → **absorb** the nested `Halt`/`Hundred`
outcome (`let _ = with_state_mut(error_in)`), since cleanup already commits to
`HISTORY_FATAL_ERROR`; `m_log`'s `x ≤ 0` branch (dead — `norm_rand`/`\normaldeviate`
always feeds `x ≥ 1`, and it runs under `conv_toks_in`'s held borrow so it
double-borrow-panics *before* `m_log`, since norm_rand's own RNG accessors
`random_entry`/`j_random` are `with_state`) → the same `let _ = with_state_mut(
error_in)` (panic-identical to the old `error()`, never executed). The lone
behavioral delta — `post_error_message`'s surfaced `format_buf` message on the
nested-Halt terminal path (OLD "halted on potentially-recoverable error as
specified" vs NEW the primary abort's message) — is **test-invisible**: `halt_on_
error_p` is the harness/tectonic default, but a passing run never fires a terminal
overflow/confusion/pdf_error, TRIP runs `halt_on_error_mode(false)`, and the message
is never written to the compared `.log`/`.xdv`. `ErrorOutcome::finish` remains (the
`SccAbort::fire` backstop); `error_in` stays the shared recoverable-error core.

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
- ~~`main_control.rs:706` (`get_preamble_token_in`) interwoven-align `fatal_error`~~
  **DONE (session 248):** threaded via the sibling `fin_align` idiom
  `return Err(MainControlAbort::Input(InputAbort::Fatal(msg)))` →
  `EngineAbort::Fatal` → `into_history`, propagated by `?` through the two call
  sites. Verified EQUIVALENT (and strictly more correct — the OLD under-borrow
  `fatal_error` would have RefCell-panicked via `into_history`'s
  `close_files_and_terminate` re-entering `with_state`, on an unreachable path).
  This was the *only* live interwoven-align longjmp; the other five sites
  (input.rs `end_token_list_in`, scan.rs `get_next`, the two `fin_align` sites)
  were already threaded.
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

**Blocker summary — RESOLVED (session 249).** As of session 249 **every longjmp
source of any kind is gone**: the crate has **0 `_tt_abort` calls, 0 `.fire()`
calls, and 0 C `ttstub_input_seek`/`close` calls**. The clusters listed here in
session 248 were all cleared:
- **`open_log_file` + `finalize_dvi_file` `(B)` pair** — folded (§6a below):
  `close_files_and_terminate` now returns `Result` (normal-exit propagates, the
  three terminal-cleanup callers absorb); the whole deferred-fire backstop tree
  collapsed with them (`fire_shipout_abort`/`SccAbort::fire`/`InputAbort::fire`/
  `OpenLogAbort::fire`/`ErrorOutcome::finish`/`node_recycle_abort`/
  `fire_name_overflow`/`fire_make_tex_string`/`fire_main_memory_overflow`/the four
  `overflow`/`confusion`/`fatal_error`/`pdf_error` `-> !` primitives — all deleted).
- **`ext.rs` native-font ×13** — threaded via `NativeFontAbort` (the 11 "bad flag"
  consistency checks + linebreak iterator, plus the font-loader mapping-read +
  input-close cascade `load_mapping_file_in`→`read_common_features_in`→`load_ot_font`
  →`find_native_font_in`→`load_native_font_in`/`read_font_info_in`).
- **the two deferred `ttstub_input_close` sites** — `load_mapping_file_in` (folded
  with the native-font cluster) and `u_close` (threaded via
  `InputAbort::InputClose`/`SccAbort::InputClose`/`OpenOrCloseAbort::InputClose`;
  `end_file_reading_in` now returns `Result`, terminal `tt_cleanup` absorbs).

**No Rust path and no bridge_core I/O path longjmps.** The setjmp is now a dead
backstop that can never fire. The flip (§6) is unblocked — gated only on the
general Phase-C `with_state` drain (≈221 sites + ≈191 `globals.rs` accessors),
not on any longjmp concern.

## 5.7 The session-249 longjmp-elimination sweep

Eight commits took the residue from 28 `_tt_abort` + 4 `.fire()` (session-248 end)
to **zero of each**:
1. **Delete the §5.6 DEAD wrappers** — `input_line`/`open_or_close`/`more_name`/
   `end_name`/`scan_and_pack_name`/`scan_file_name`/`scan_file_name_braced` (io.rs),
   `make_src_special` (texmfmp.rs), + the now-dead `OpenOrCloseAbort::fire` /
   `fire_make_src_pool_overflow`. −1 `_tt_abort`.
2. **`open_log_file` Result/terminal callers** — `final_cleanup` propagates,
   `pre_error_message` + `SccAbort::into_engine_abort` NoLegalEnd absorb.
3. **`finalize_dvi_file` fold → the deferred-fire spine collapse** —
   `close_files_and_terminate` → `Result`; the whole `fire`/`-> !` backstop tree
   deleted. `.fire()` 4 → 0, `_tt_abort` 27 → 15.
4. **The two non-native inline `_tt_abort`** — `ship_out_in` CannotOpenOutput (a
   latent under-borrow longjmp) → `Err(ShipoutAbort::CannotOpenOutput)`;
   `check_pool_pointer` (dead) deleted. 15 → 13.
5. **Native-font measurement ×12** — `NativeFontAbort`; `_tt_abort` 13 → 1.
6. **Font-loader mapping-file cascade** — the last `_tt_abort` → 0.
7. **`u_close`** — the last bridge_core input longjmp → 0.

`unsafe fn` 260 → ~230; warnings 37 throughout; dashboard 29/2/7/4 per commit;
corpus 637/2 (known-FP only). The `-> !` fatal primitives, the deferred-fire
`fire()` dispatchers, and the C input-I/O longjmp wrappers are all gone from every
reachable path; `EngineAbort::into_history` is the sole terminal, `Result` the sole
channel to `tt_run_engine`.

## 6. Final flip (all-or-nothing, later session)

**Preconditions MET (session 249): no Rust path and no bridge_core I/O path
longjmps** — 0 `_tt_abort`, 0 `.fire()`, 0 `ttstub_input_seek`/`close`. The setjmp
is a dead backstop. The flip is now gated **only** on the general Phase-C
`with_state` drain (≈221 `with_state`/`with_state_mut` sites + ≈191 `globals.rs`
accessors + the 41 `mod raw` scalars must be threaded off the `thread_local!
RefCell` first, so that owning `EngineState` in `tt_run_engine` leaves no live
`with_state` leaf to double-borrow) — not on any longjmp concern. When that drain
completes:

- `tt_run_engine` owns `let mut st = EngineState::new();` and threads `&mut st`
  everywhere; its body becomes `match inner(&mut st) { Ok(h) => h, Err(a) =>
  a.into_history() }`.
- `xetex-engine-interface.c` drops the `setjmp`; `tt_run_engine`'s `i32` return is
  the sole channel.
- `with_state`/`with_state_mut`, the `thread_local! RefCell`, the ~98 `globals.rs`
  wrappers, and the 41 `mod raw` scalars all delete **together** (one commit) —
  there is no live `with_state` leaf left to nest-panic under the owned `&mut`.

This is gated, not scheduled: it lands only when both preconditions hold.

## 6.1 The `with_state` drain roadmap (session 249 frontier map)

A 5-agent frontier map (session 249, all claims source-verified) settled the *shape*
and *order* of the post-longjmp drain toward §6. Reconciled counts: **221** real
`with_state`/`with_state_mut` sites — `globals.rs` 98 / `format_io.rs` 35 / `engine.rs`
32 / `output.rs` 17 / `io.rs` 14 / `texmfmp.rs` 7 / rest ≤4. The leaf subsystems
(`main_control` 1, `scan` 1, `math`/`packaging` 0…) are already threaded; the residue
is concentrated in the boundary/wrapper layer that the setjmp gated.

**Shape: incremental drain → ONE atomic final flip.** Not one giant commit.
- **Drain (safe, per-commit, RefCell alive throughout):** the established sessions-232–248
  pattern (with_state 352→221; globals wrappers 198→98). A boundary calls
  `with_state_mut(|st| core_in(st))`, taking one borrow for the whole callee and
  threading `&mut st` down; converting a leaf to `&mut EngineState` deletes its inner
  `with_state` and pushes the borrow point up. Only one borrow is ever live per path,
  so nothing nests. A `globals.rs` accessor deletes the moment its last external caller
  threads.
- **Final flip (genuinely atomic, §6):** in ONE commit `tt_run_engine` swaps its
  scattered `with_state_mut` blocks for `let mut st = EngineState::new()` and deletes
  `with_state`/`with_state_mut` + the `thread_local! RefCell` (`state.rs:1455–1467`) +
  the ~98 `globals.rs` wrappers, folds in the `mod raw` scalars, drops the dead setjmp.
  Cannot be split: an owned local `st` alongside any surviving external `with_state`
  leaf ⟹ that leaf reads the *orphaned* thread-local (silent split-brain), and deleting
  `with_state` is a compile-break unless zero external callers remain.
- **Exact flip gate:** *zero `with_state`/`with_state_mut` outside `tt_run_engine`'s own
  frame.* NOT "zero everywhere first" — the ~32 sites lexically inside `tt_run_engine`
  (engine.rs:1067…1471) convert atomically *in* the flip commit itself.

**Ordered roadmap (dependency-sorted):**
1. **`get_strings_started`** (engine.rs:149) — pilot; 3 inline sites + `set_pool_ptr`/
   `set_str_ptr`/`pool_size` calls → `st.strings.*`; 1 caller (INITEX init). **DONE
   session 249** (`fe16b82d`, with_state 221→219).
2. **`close_files_and_terminate`** (io.rs) — 5 sites, all workers threaded
   (`finalize_dvi_file_in`, `synctex_terminate(st,…)`, `print_*_in`); already `Result`.
   **Prerequisite for `store_fmt_file`**; its four callers (normal-exit + errors.rs:99/294
   post-borrow + format_io.rs:260) all hold no borrow, so bridge cleanly. **DONE
   session 250** (`c6c79fe6`, 219→215): `close_files_and_terminate_in(st)` + a thin
   one-`with_state_mut` bridge wrapper keeping the four callers unchanged.
3. **`tt_cleanup`** (engine.rs) — 5 sites, no Result friction; two big single-borrow blocks.
   **DONE session 250** (`cd1b2142`, with_state 215→209): `tt_cleanup_in(st)` + a thin
   one-`with_state_mut` bridge wrapper (3 callers — engine.rs normal-exit + errors.rs:100/295
   post-borrow — unchanged); the font-loop `font_area_entry`/`font_mapping_ptr` reads and the
   `set_font_mapping_ptr`/`deinitialize_shipout_variables` wrappers fold to direct `st.*`,
   deleting both now-dead wrappers. Re-entrant calls are all pure FFI, sound under the borrow.
4. **`load_fmt_file`** (format_io.rs) — trivial once `pack_buffered_name` gets a one-line
   `_in`; `load_fmt_file_in` is already the flip's reference model.
   **DONE session 250** (`9e49c308`, with_state 209→208): the pack-name prelude now runs
   inside `load_fmt_file`'s single `with_state_mut` — `pack_buffered_name` inlined onto the
   borrow (name_of_file/name_length written directly), the file open reads `st.input.name_of_file`
   directly, `cur_input()`/`set_cur_input_loc` stay (mod-raw scalars, borrow-independent).
   Deletes now-dead `pack_buffered_name` + its redundant `tre_format_default_length` extern.
   The frame is now fully drained (zero with_state re-entries — the flip-gate condition).
5. **`store_fmt_file`** (format_io.rs) — the big drain (~34 sites), mechanically uniform;
   after #2 (it calls `close_files_and_terminate`).
   **DONE session 250** (`fbb1d488`, with_state 208→161): `store_fmt_file_in(st)` + thin
   wrapper — the whole ~600-line dump under one borrow. All ~40 with_state re-entries →
   direct `st.*`; every `print_*`/`output::` → `_in`; the two `error()` sites call
   `error_in(st)`; `pack_job_name`/`make_name_string`/`close_files_and_terminate`/`init_trie`
   use their `_in` cores (`map_err` into EngineAbort). The eqtb-run/hash-word helper
   closures now take `st` as a parameter (capture nothing) so no immutable borrow outlives
   the mutable tail. **Cascade landed early:** `store_fmt_file` was the last non-`_in`
   caller of `print`/`print_ln`/`print_esc`/`print_file_name` (output.rs), 10 strings/
   font/save globals accessors, and `init_trie` (hyphenation.rs) — all 15 deleted.
6. **`final_cleanup`** (engine.rs:309) — after #5; its input-SCC calls
   (`end_token_list`/`end_file_reading`) return `InputAbort`, so bare `?` becomes
   `map_err(|e| e.into_engine_abort())?`.
   **DONE session 250** (`14fc2aa5`, with_state 161→142): `final_cleanup_in(st)` + thin
   wrapper. All with_state re-entries → direct `st.*`; `print_*`/`output::` → `_in`
   (incl. `show_save_groups_in`/`print_cmd_chr_in`); `end_token_list_in`/
   `end_file_reading_in`/`open_log_file_in` with `map_err`; the c==1 INITEX dump block
   inlines its former `with_state_mut` closure and calls `store_fmt_file_in(st)`;
   `flush_node_list`'s `&CStr` maps to `EngineAbort::Confusion`. **Cascade:** last non-`_in`
   caller of `print_esc_cstr`/`print_nl`/`print_cmd_chr`/`show_save_groups`, the
   `store_fmt_file`/`end_token_list`/`end_file_reading` wrappers, and `mem_b32s1` — 8 deleted.
7. **Cascade:** with #1–6 done the ~140 thin wrappers lose their last un-threaded callers
   and delete with their bodies — the 5 boundary-only init/deinit wrappers
   (`initialize_pagebuilder_variables`, `initialize/deinitialize_shipout_variables`,
   `init_trie`, `pack_buffered_name`), the output/texmfmp/io/scaled_math leaves, and the
   98 `globals.rs` accessors becoming caller-free.
8. **Residue:** 5 post-borrow re-entries (scan.rs:2351, errors.rs:70/94, fonts.rs:169/1059)
   need small `_in` pulls; the C-ABI entries (`font_feature_warning` output.rs, and
   `tt_xetex_open_input` io.rs) need a raw-pointer stash rather than the borrow.
9. **FINAL FLIP** (one commit, §6).

**Traps (session-249 map, verified):**
- **`mod raw` scalars STAY PUT during the drain.** Do NOT fold
  `cur_cs`/`cur_tok`/`cur_chr`/`cur_cmd`/`cur_val`/`cur_ptr`/`line`/… (globals.rs
  category-B, ~92 fns) into `EngineState` until the flip — they are the borrow bridge
  (`main_control.rs:23–26` reads them under the live `main_control_in` borrow). Folding
  one while any `with_state` leaf survives creates a held-`&mut` conflict.
- **No `is_char_node`/`hi_mem_min` freshness hazard** in any boundary fn (the lone
  `hi_mem_min` write at format_io.rs:764 is a plain field store). The real friction is
  **split-borrow**: a folded site sometimes needs two disjoint sub-structs at once
  (`errors`+`strings`, `input`+`shipout`) — take per-field reborrows the single-field
  wrapper hides.
- **Config carry-over (flip-only, easy to miss):** `tt_xetex_set_int_variable`
  (engine_interface.rs:11–34) writes 5 config fields (`halt_on_error_p`,
  `in_initex_mode`, `synctex_enabled`, `semantic_pagination_enabled`,
  `shell_escape_enabled`) into the thread-local *before* `tt_run_engine` runs, so a fresh
  `EngineState::new()` in the flip would discard them — stash in retained statics and read
  into `st` at startup.

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
