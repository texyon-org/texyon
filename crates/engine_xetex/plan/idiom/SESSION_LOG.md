# Idiom session log

Recent Stage 2 idiomatization handoff log. Newest entry is at the top.
Read this file first in new sessions; only open archives when investigating older context.

Archives:
- `session-log/sessions-100-149.md`
- `session-log/sessions-050-099.md`
- `session-log/sessions-001-049.md`

## Idiom session 251 — 2026-07-02 — Execute the §6 flip via a pointer bridge (01d §6.2); with_state 142→15

**Update (cont.):** after the boundary-layer flip (below), the drain continued through the
leaf frontier to **`with_state` 15** (~15 commits total, all dashboard 29/2/7/4 + 80 unit +
36 warn; corpus 637/2): random/log math cluster + arith-test refactor + `get_microinterval`
+ `load_fmt_file`/`init_io`/`init_start_time`/`initialize_*_variables` + cascade-deletion of
dead wrappers (culminating in `tt_xetex_open_input` → 4 output wrappers). **The remaining ~15
is the practical floor: C++ callbacks (`font_feature_warning`/`gr_font_get_named`) that call
BACK into Rust are `#[no_mangle]` with no `st` — they MUST use the bridge, so true
zero-`with_state` is architecturally unreachable.** The bridge (pointer to the owned `st`) is
the legitimate end-state; the fallback `RefCell` is never touched in production. Rest of the
15: the `into_engine_abort` diagnostic-count pin (design change, not 96 sites) + deep `hash_s1`.
The §6 flip is COMPLETE — `EngineState` is owned; the engine threads `&mut st`. Full detail:
`01d` §6.2, [[engineabort-unification-progress]].


**Type:** execution — the §6 flip (owned `EngineState`). Instead of the "one atomic
commit" the roadmap feared, a **staged pointer bridge** made it incremental and
validatable: `tt_run_engine` OWNS its `EngineState` (heap-boxed local) and installs a
thread-local raw pointer; `with_state`/`with_state_mut` reach `st` through it, falling
back to the old thread-local `RefCell` only when the pointer is null (unit tests +
pre-run `tt_xetex_set_int_variable`, which also keeps that path's double-borrow panic
net). Each `with_state` region then drained to direct `&mut EngineState` threading, one
validated commit at a time.

**8 commits, each dashboard 29/2/7/4 + 80 unit tests + 37 warnings** (corpus 637/2 on
the bridge pivot `0ee31a4a`; a clean corpus on the final state was running at handoff):
1. `0ee31a4a` — bridge + owned `st` + config carryover (behavior-identical).
2. `80ec07a6` — `tt_run_engine`'s 13 in-frame `with_state_mut` blocks → owned `st`.
3. `54838c23` — all ~65 `tt_run_engine` accessor calls → `st.*`; **57 dead accessors deleted**.
4. `3f6c2bbb` + `8afaa725` — `tt_run_engine`'s boundary calls go to the `*_in` cores
   (`main_control_in`/`final_cleanup_in`/`close_files_and_terminate_in`/`tt_cleanup_in`/
   `start_input_in`/`maketexstring_in`); `final_cleanup`/`main_control`/7 io wrappers/
   `maketexstring` wrapper deleted.
5. `03ed6003` — **`into_history(self, st)`** + `pre_/post_error_message_in` thread the
   whole terminal-cleanup tree; 12 more dead accessors deleted.
6. `f123d963` — host config moved to process atomics (off the state); `apply_config`.

**`with_state` 142 → 48.** The ENTIRE boundary/terminal layer (`tt_run_engine`,
`into_history` + `pre_/post_error_message`, every `*_in` boundary fn) is now threaded
with zero `with_state`; `EngineState` is owned. **Remaining 48** = leaf re-entries in
not-yet-threaded functions (the general Phase-C frontier: ~21 globals accessors +
output/texmfmp/scaled_math wrappers). Removing the bridge + fallback entirely (true
zero-`with_state`) is now a pure mechanical cascade with NO architectural blocker.
Full detail: `01d` §6.2, [[engineabort-unification-progress]].

## Idiom session 250 — 2026-07-01 — Map the post-longjmp `with_state` drain frontier + pilot slice (01d §6.1)

**Type:** understand + execution. A 5-agent frontier-map workflow (all claims
source-verified) + the pilot drain slice. Answers "can we now continue threading?" —
**yes**, and writes down the how.

**The map (→ `01d` §6.1).** With every longjmp gone (session 249), the only thing that
forbade a boundary fn from holding a long-lived `&mut EngineState` — a `longjmp`
discarding the `RefCell` borrow guard (`state.rs:1461`) without `Drop`, poisoning the
cell — is gone. The 221 `with_state` sites concentrate in the boundary/wrapper layer
(`globals.rs` 98 / `format_io.rs` 35 / `engine.rs` 32); the leaf subsystems are already
threaded. Shape: **incremental drain → ONE atomic final flip** (the flip can't be split —
an owned local `st` beside any surviving external `with_state` leaf = split-brain). Exact
flip gate: **zero `with_state` outside `tt_run_engine`'s own frame**. Ordered 9-step
roadmap + traps (mod-raw scalars stay put; split-borrow not `is_char_node` freshness; the
`tt_xetex_set_int_variable` config carry-over must be stashed in retained statics at the
flip) — all in `01d` §6.1.

**Pilot (`fe16b82d`).** `get_strings_started` → `get_strings_started_in(st: &mut
EngineState) -> Result<(), EngineAbort>`: 3 inline `with_state` + the `set_pool_ptr`/
`set_str_ptr`/`pool_size` wrapper calls → direct `st.strings.*`; sole caller
(`tt_run_engine` INITEX init) bridges via one `with_state_mut(|st| …_in(st))`. Dropped
the now-unused imports. `with_state` **221 → 219**; warnings 37; dashboard 29/2/7/4
(TRIP's `\dump` exercises this INITEX path).

**Drain executed — roadmap steps 2–6 all DONE this session (each dashboard-green 29/2/7/4,
37 warnings; corpus 637/2 validated cumulatively).** Each boundary fn became
`fn_in(st: &mut EngineState)` + a thin `with_state_mut` wrapper, fully drained:
- **2. `close_files_and_terminate`** (`c6c79fe6`) — 219→215.
- **3. `tt_cleanup`** (`cd1b2142`) — 215→209; deleted dead `set_font_mapping_ptr` + `deinitialize_shipout_variables`.
- **4. `load_fmt_file`** prelude (`9e49c308`) — 209→208; `pack_buffered_name` inlined + deleted.
- **5. `store_fmt_file`** (`fbb1d488`) — the big ~600-line drain, 208→**161**; all `print_*`→`_in`,
  eqtb-run/hash-word helper closures reparametrized to take `st`; cascade deleted 15 now-dead
  wrappers (4 output primitives + 10 globals accessors + `init_trie`).
- **6. `final_cleanup`** (`14fc2aa5`) — 161→**142**; c==1 INITEX block inlines its `with_state_mut`
  closure + calls `store_fmt_file_in(st)`; cascade deleted 8 more (`print_esc_cstr`/`print_nl`/
  `print_cmd_chr`/`show_save_groups`, `store_fmt_file`/`end_token_list`/`end_file_reading` wrappers, `mem_b32s1`).

`crate::state::with_state` **221 → 142** (−79). **The ordered boundary-fn drain (steps 1–6) is
complete.** Remaining `with_state`: 86 `globals.rs` accessor *definitions* (delete at the flip) +
~15 inside `tt_run_engine`'s own init frame + ~40 scattered non-`_in` wrapper bodies pinned by
that frame. Draining these = threading `tt_run_engine` itself = **step 9, the atomic FLIP** (delete
`with_state`/`thread_local`/setjmp together + re-plumb `tt_xetex_set_int_variable`'s 5 config
fields into retained statics). That's the user-gated major change; paused here for direction.
See [[globals-wrappers-gated-on-setjmp-boundary]], [[engineabort-unification-progress]].

## Idiom session 249 — 2026-07-01 — Execution: eliminate EVERY remaining longjmp — 0 `_tt_abort`, 0 `.fire()`, 0 bridge_core input-close (01d §5.6/§5.7/§6)

**Type:** execution (`01d` §5.6 → §6 unblock). Autonomous: **8 commits**, dashboard
29/2/7/4 per commit, corpus 637/2 (known-FP only) validated.

**The job.** Session 248 left the longjmp residue at 28 `_tt_abort` + 4 `.fire()`.
This session drove it to **zero of every longjmp kind** — no Rust `_tt_abort`, no
deferred-fire `.fire()`, no C `ttstub_input_seek`/`close` — so the setjmp is now a
dead backstop and the §6 flip is unblocked (gated only on the general `with_state`
drain, not longjmp).

**The seven levers (see `01d` §5.7 for the full census):**
1. **DEAD wrapper deletion** — the §5.6 DEAD set (`input_line`/`open_or_close`/
   `more_name`/`end_name`/`scan_*_name` in io.rs, `make_src_special` in texmfmp.rs)
   + the now-dead `OpenOrCloseAbort::fire`/`fire_make_src_pool_overflow`.
2. **`open_log_file` `(B)` fold** — `final_cleanup` propagates via `?`;
   `pre_error_message` + `SccAbort::into_engine_abort` NoLegalEnd absorb (mirroring
   session-248's `post_error_message` absorb).
3. **`finalize_dvi_file` fold → deferred-fire spine collapse (the big one)** —
   `close_files_and_terminate` → `Result<(), EngineAbort>` (normal-exit propagates
   via `into_history`; the three terminal-cleanup callers absorb). With
   `finalize_dvi_file()` gone the whole backstop tree became unreachable and was
   deleted: `fire_shipout_abort`, `SccAbort::fire`, `InputAbort::fire`,
   `OpenLogAbort::fire`, `ErrorOutcome::finish`, `node_recycle_abort`,
   `fire_name_overflow`, `fire_make_tex_string`, `fire_main_memory_overflow`, the
   `open_log_file` wrapper, and the four `overflow`/`confusion`/`fatal_error`/
   `pdf_error` `-> !` primitives. `.fire()` **4 → 0**.
4. **Two non-native inline `_tt_abort`** — `ship_out_in` CannotOpenOutput (a latent
   under-borrow longjmp) → `Err(ShipoutAbort::CannotOpenOutput)`; dead
   `check_pool_pointer` deleted.
5. **Native-font measurement ×12** — new `NativeFontAbort` leaf; the 11 "bad native
   font flag" consistency checks (unreachable — OK path byte-identical, callers gain
   `?`) + `linebreak_start_in`'s iterator error, threaded through ~40 sites via
   `From<NativeFontAbort>` on SccAbort/MainControlAbort/ShipoutAbort/MathBuildAbort/
   NativeCharAbort/NodeRecycleAbort/HyphNativeAbort, plus the non-Result
   intermediaries `set_native_metrics`/`store_justified_native_glyphs`.
6. **Font-loader mapping-file cascade** — the last `_tt_abort`
   (`load_mapping_file_in` "could not read mapping file" + its deferred input-close)
   threaded through `read_common_features_in`→`load_ot_font`→`find_native_font_in`→
   `load_native_font_in`/`read_font_info_in` (each single-/few-return → `Result`);
   deleted the dead `readCommonFeatures` `#[no_mangle]` C-ABI export. `_tt_abort` **→ 0**.
7. **`u_close`** — the last bridge_core input longjmp → `io::input_close` + bool;
   `end_file_reading_in` → `Result<(), InputAbort>` (new `InputAbort::InputClose`),
   threaded through read_toks/get_next/final_cleanup; `\read`/`\closein` closes →
   `SccAbort`/`OpenOrCloseAbort::InputClose`; terminal `tt_cleanup` absorbs.

**Behavior preservation.** Every terminal maps to the same `EngineAbort` arm the old
longjmp produced (`into_history` = same prints/cleanup/history 3/surfaced message).
The recurring absorb pattern (terminal cleanup drops a *second* error, already
committed to `HISTORY_FATAL_ERROR`) is the session-248 `post_error_message` idiom.
The 11 "bad flag" branches are unreachable, so all exercised paths are byte-identical
— corpus 637/2 confirms.

**Metrics:** `_tt_abort` **28 → 0**, `.fire()` **4 → 0**, `ttstub_input_seek/close`
calls **→ 0**; `unsafe fn` 260 → ~230; warnings **37**. Commits `6654b005`,
`eee12af1`, `d6fc9ccc`, `a20373a3`, `7ca7f43e`, `8d92f9f0`, `868d0efa` (+ docs).

**Next (`01d` §6).** The flip is unblocked. It remains all-or-nothing and large:
thread the ≈221 `with_state`/`with_state_mut` sites + ≈191 `globals.rs` accessors +
41 `mod raw` scalars off the `thread_local! RefCell`, then `tt_run_engine` owns
`EngineState`, `xetex-engine-interface.c` drops the (now-dead) setjmp, and the
global-state machinery deletes together. No longjmp concern gates it any more — it is
the general Phase-C drain.

## Idiom session 248 — 2026-07-01 — Execution: the two most tractable Rust `_tt_abort` residue pieces — interwoven-align + `error()`

**Type:** execution (`01d` §5.6). Autonomous: three commits + a 6-agent adversarial
verify (**6/6 EQUIVALENT**). Threaded the two most tractable Rust longjmp residue
pieces gating the §6 setjmp flip: the `main_control` interwoven-align inline
`fatal_error`, and the whole `error()` recoverable-error wrapper (deleted).

**The job (`01d` §5.6).** After Stage I/II, the Rust `_tt_abort` spine gating setjmp
deletion was down to `error()` + `open_log_file`/`finalize_dvi_file` (B-class) + two
inline classes (interwoven-align, ext native-font ×13). This session took the two
smallest levers.

**Commit 1 (`ceea6093`) — interwoven-align.** `get_preamble_token_in`'s `ENDV`
check called `crate::errors::fatal_error("(interwoven alignment preambles are not
allowed)")` **directly under the `&mut EngineState` borrow** — the last live inline
longjmp on main_control's alignment path (distinct from the folded deferred
`MainControlAbort` spine). Recon found the other **five** "interwoven alignment"
sites already threaded (input.rs `end_token_list_in`, scan.rs `get_next`, the two
`fin_align` sites at main_control.rs ~881/~1599 via `MainControlAbort::Input(
InputAbort::Fatal(...))`), so this was the sole holdout. Replaced it with that exact
sibling idiom: `return Err(MainControlAbort::Input(InputAbort::Fatal(msg)))` →
`into_engine_abort` → `EngineAbort::Fatal(msg)` → `into_history` ("Emergency stop" +
msg + inline cleanup, returns 3). Propagated via `?` through the two
`get_preamble_token_in` call sites. **Verifier finding:** the OLD under-borrow
`fatal_error` would have *RefCell-panicked* if ever reached (`into_history`'s
`close_files_and_terminate` re-enters `with_state`), so NEW is strictly more correct
on that cold path — but it is unreachable by TRIP/corpus, so no exercised-path change.

**Commit 2 (`a9030166`) — the two `store_fmt_file` `error()` sites.** The
`\dump`-inside-a-group refusal and the native-fonts/font-mappings dump refusal both
called the global `error()` wrapper (whose `.finish()` longjmps `_tt_abort` on the
Halt/Hundred outcome). Threaded each: run `error_in` under a fresh borrow, and on a
non-`Continue` outcome `return Err(outcome.into_engine_abort())`. Continue is
unchanged (dump-in-group falls through to its `TtAbort` terminal; native-fonts
resumes the font-dump loop); Halt/Hundred → `EngineAbort::HaltMessage` → the same
`post_error_message(false)` (one `close_files_and_terminate`) + surfaced halt message,
history 3. Both fns already return `Result<(), EngineAbort>`.

**Commit 3 (`0cd792f4`) — the last two `error()` callers + delete `error()`.**
- `scaled_math.rs` `m_log` `x<=0`: a **dead** branch (its only caller `norm_rand`
  always passes `x >= 1`) that additionally runs under `conv_toks_in`'s held borrow
  (norm_rand's RNG accessors — `random_entry`/`j_random` in globals.rs — are
  `with_state`, so the path double-borrow-panics *before* `m_log`). Replaced `error()`
  with `let _ = with_state_mut(error_in)`: same panic-if-reached at the same
  `with_state_mut`, never executed, no longjmp. Dropped the now-unused
  `use crate::errors::error`.
- `errors.rs` `post_error_message`: runs inside `into_history`'s terminal cleanup
  (post-borrow), which unconditionally commits to `HISTORY_FATAL_ERROR` immediately
  after. **Absorb** a nested Halt/Hundred outcome (drop it) rather than longjmp.
- With all four callers threaded, deleted `pub unsafe fn error()` (`error_in` stays
  the shared core; `ErrorOutcome::finish` remains for `SccAbort::fire`).

**Adversarial verification (Workflow, 6 skeptics, ultracode).** One agent per
converted path (interwoven-align, store_fmt ×2, m_log dead-branch, post_error_message
absorb, deletion cleanliness), each re-deriving OLD (baseline `f3b264ba`) vs NEW.
**Result: 6/6 EQUIVALENT.** The one flagged behavioral difference — the
**post_error_message absorb**'s surfaced-message change on the nested Halt/Hundred
terminal path (OLD "halted on potentially-recoverable error as specified" vs NEW the
primary abort's "halted on overflow()"/…) — is **test-invisible**: `capture_to_
diagnostic(null)` is idempotent and `close_files_and_terminate`/`tt_cleanup` still run
exactly once, and although `halt_on_error_p` is the harness/tectonic *default* (so the
path is not merely the rare 100th-error case — audit correction to the recon), a
passing run never fires a terminal overflow/confusion/pdf_error, TRIP runs
`halt_on_error_mode(false)`, and the surfaced `format_buf` string is never written to
the compared `.log`/`.xdv`. Same acceptance class as the Stage II close-message delta.

**Metrics:** `unsafe fn` **260 → 259** (`error()` deleted; the `into_engine_abort`
threading adds none); warnings **37**; `with_state` unchanged. Validation: dashboard
**29/2/7/4** per commit (TRIP 2/2 — exercises the recoverable-error machinery); release
rebuild; full corpus **637 passed / 2 diverged** (same known-FP pair, doc-count 639
unchanged). Commits `ceea6093`, `a9030166`, `0cd792f4`.

**Next (`01d` §6).** The Rust `_tt_abort` residue is now down to `open_log_file` +
`finalize_dvi_file` (the two B-class deferred-fire wrappers with terminal-machinery
callers) + the `ext.rs` native-font ×13 inline class (which also unblocks the deferred
`load_mapping_file_in` close). The bridge_core input-I/O side still longjmps only from
the two deferred `ttstub_input_close` sites (`u_close`, `load_mapping_file_in`). Only
once **no** Rust path AND **no** bridge_core I/O path longjmps does the setjmp +
owned-`EngineState` flip (§6) land.

## Idiom session 247 — 2026-07-01 — Execution: Stage II — the input-I/O longjmp sweep (ttstub_input_seek/close → ttbc_*/Result)

**Type:** execution (Stage II, `01d` §2/§2.1). Autonomous: six commits + an 11-agent
adversarial verify (**11/11 EQUIVALENT**).

**The job (`01d` §2).** Neutralize longjmp sources (2)/(3) of the three feeding the
xetex setjmp — the C `ttstub_input_seek`/`ttstub_input_close` wrappers, which
`longjmp` *from C* on a serious internal I/O error. (Source (1), the Rust `_tt_abort`
spine, was converted-or-enumerated in Stage I / session 246.)

**Recon → decision (`01d` §2.1): Option A.** Call the exported C-ABI
`ttbc_input_seek`/`ttbc_input_close` directly (they return the error as a value,
never jump), getting `&mut CoreBridgeState` from the existing `_ttbc_get_core_state()`
accessor. **Decisive:** `CoreBridgeState::input_seek` is a **private `fn`** in
bridge_core, so Option B ("call the Rust method") isn't even in-crate for seek — only
the exported `ttbc_*` symbols are reachable. B would also thread a second long-lived
`&mut CoreBridgeState` through the whole deep I/O tree alongside the `with_state`
RefCell (large blast radius, nothing gained). `io::input_seek`/`io::input_close` wrap
the two ttbc calls.

**Converted 15 of 17 sites (6 commits; dashboard 29/2/7/4 each, warnings 37):**
- **Dead-code (`7870b9b5`):** deleted `u_open_in` + `set_input_file_encoding` (0 live
  callers) — drops 3 seek sites for free.
- **Seek (`02cb5a9b`):** all 7 seek sites → `io::input_seek`. Every site rewinds with
  `SEEK_SET`, and `ttbc_input_seek` flags `internal_error` only for an invalid `whence`,
  so the seek longjmp is **provably unreachable** (asserted) — no abort arm needed.
- **Close, Result-direct (`4ec64abf`):** `load_fmt_file_in` (→ `EngineAbort::TtAbort`)
  + `read_font_info_in` (→ new `ReadFontInfoAbort::InputClose`), enclosing fns already
  return `Result`.
- **Close, conv_toks trio (`0a4b3b6b`):** `getfilesize_in`/`getfiledump_in`/
  `getfilemoddate_in` → `Result<(), SccAbort>` (new `SccAbort::InputClose`), propagated
  via `?` through their sole live caller `conv_toks_in`; deleted the 3 dead bridge
  wrappers.
- **Close, pic (`b149a1f9`):** `find_pic_file` → `Result<c_int, MainControlAbort>`
  (new `MainControlAbort::InputClose`), `count_pdf_file_pages_named` →
  `Result<c_int, SccAbort>`, propagated via `?` through `load_picture_in` /
  `scan_something_internal_in`.
- (`2e450322`: rustfmt line-wrap of the `input_seek` ttbc call — no behavior change.)

Each close site: on `ttbc_input_close != 0` (the exact condition the C wrapper
`longjmp`'d on — `input_close`'s `scan_remainder` failed) return
`Err(<Subsystem>Abort::InputClose)` → `EngineAbort::TtAbort` → `tt_run_engine` →
`into_history` (history 3). The old bare `longjmp` (support.c:332, **not** `_tt_abort`)
ran no cleanup and left `format_buf` undefined; `TtAbort` runs no cleanup, sets a
descriptive message, returns 3 — control-flow identical, the message a documented,
test-invisible improvement.

**Deferred (2 sites, with rationale — `01d` §2.1 table):** `u_close` (io.rs) —
**(B)-class**: its close-abort is reached from `close_files_and_terminate` inside
`into_history`'s terminal machinery (a §6-flip concern), alongside threadable callers.
`load_mapping_file_in` (ext.rs) — folds with the **ext.rs native-font ×13
inline-`_tt_abort` class** (§5.6), which threads only when the measurement layer grows
an error channel. The `xmalloc` `_tt_abort` fold stays deferred (OOM terminal, §2).

**Adversarial verification (Workflow, 11 skeptics, ultracode).** One agent per
conversion class + the ABI/terminal infra, each re-deriving the old ttstub longjmp vs
the new ttbc/Result path (extern ABI compat vs bridge_core, `SEEK_SET`
unreachability, close terminal = `TtAbort`/no-cleanup/history-3, guard preservation).
**Result: 11/11 EQUIVALENT, 0 divergent.** These I/O-error paths are exercised by
neither the dashboard nor the corpus (they need a mid-read internal failure), so the
verify was the primary correctness gate.

**Metrics:** `unsafe fn` **263 → 260** (−3 dead wrappers; the InputClose variants add
zero unsafe fns); `with_state` unchanged; warnings **37**. Validation: dashboard
**29/2/7/4** per commit; release rebuild; full corpus **637 passed / 2 diverged**
(known FP pair). Left the pre-existing `format_io.rs` compact one-liners un-fmt'd (as
prior sessions). Commits `7870b9b5`, `02cb5a9b`, `4ec64abf`, `0a4b3b6b`, `b149a1f9`,
`2e450322`.

**Next (`01d` §6).** Two `ttstub_input_close` longjmp sites remain (the deferred
`u_close` + the `ext` native-font class). The Rust `_tt_abort` residue still to thread
before the flip: `error()` + `open_log_file`/`finalize_dvi_file` + the two inline
classes (§5.6). Only once **no** Rust path AND **no** bridge_core input-I/O path
longjmps does the setjmp + owned-`EngineState` flip (§6) land.

## Idiom session 246 — 2026-07-01 — Execution + census: the Stage-I residue (convert the (A) scatter, census the rest)

**Type:** execution + census (continuation of the session-245 EngineAbort lever).
Autonomous: four commits + a 14-agent adversarial workflow (7 verify, 7 census).

**The job (`01d` §5.6).** After the `main_control` fold, the remaining Rust
`_tt_abort` spine was the **non-`main_control` scatter** (~15 global wrappers +
inline longjmps, varying in reachability). This session **converted the whole
directly-propagatable `(A)` set** and **produced the census** (the deliverable).

**Enabling finding — the C core is gone.** `xetex-xetex0.c` is a 10-line empty
stub (all translated to Rust); the ~10 remaining `xetex/*.c` files are 10–101-line
helper stubs that never call *back* into the Rust wrappers, and every wrapper is
plain `pub unsafe fn` (Rust-ABI, so C can't call it). **So reachability = pure
Rust caller analysis**, and a fire-site blocks the setjmp only if a live Rust path
reaches it. This reframed the census: most "bridge-down wrappers for legacy
callers" are now **DEAD** (their legacy C callers are gone).

**Converted (8 sites, 4 commits; dashboard 29/2/7/4 each, `unsafe fn` 263,
warnings 37):**
1. **`start_input` → `Result<(), EngineAbort>`** (`21409112`) — its sole caller
   `tt_run_engine` catches with `if let Err(a) = start_input(..) { return
   a.into_history(); }` before `set_history`/`main_control`. The exact
   `main_control`-fold idiom; a direct −1 longjmp.
2. **The two `node_recycle` catch-all arms** (`037b5dd8`) — `get_strings_started`
   (engine.rs) + `store_fmt_file` (format_io.rs) `Err(e) => node_recycle_abort(e)`
   → `Err(e.into_engine_abort())`; both enclosing fns already return `Result`.
   `store_fmt_file`'s special-cased string-count `Overflow` arm ("number of
   strings") left untouched.
3. **Four single-caller wrapper-flips** (`5137505c`) — `pack_job_name`,
   `make_name_string`, `init_trie` (all sole-caller `store_fmt_file`),
   `end_token_list` (sole-caller `final_cleanup`) → `Result`, callers use `?`.
   Deleted the now-dead `trie_abort` dispatcher.
4. **`maketexstring` → `Result`** (`e24dd55b`) — the census audit **corrected the
   recon**: its two callers are `tt_run_engine`-direct (INITEX `format_ident` +
   `nullfont_name`), NOT `initialize_primitives` (which uses `maketexstring_in`),
   so it is a trivial `into_history` catch like `start_input`, no
   `PrimitiveInitAbort` plumbing.

**Adversarial verification (Workflow, 14 agents, ultracode).** 7 verify agents
(one per conversion, each re-deriving the terminal/message/side-effects/control-
flow vs the original `fire()`/dispatcher at `19a7eaeb`): **7/7 EQUIVALENT, 0
divergent** (the 8th, `maketexstring`, is transitively covered — its leaf
`MakeTexStringAbort::into_engine_abort` was verified via `pack_job_name`, its
catch is the verified `start_input` idiom). 7 census agents classified every
remaining fire-site; two returned **evidence corrections** that improved the
census (see below).

**The census (`01d` §5.6 — the deliverable).**
- **DEAD (9), do NOT block the setjmp:** `input_line`, `open_or_close`,
  `more_name`, `end_name`, `scan_and_pack_name`, `scan_file_name`,
  `scan_file_name_braced`, `make_src_special`, `set_input_file_encoding` (sole
  caller `u_open_in` itself dead). Superseded by their `_in` cores; 0 call-sites
  (verified). Deletable now or at the flip.
- **`(A)` non-trivial (1): `error()`.** Audit **corrected** the recon: it has NO
  genuinely un-threadable caller — its `m_log` "blocker" is a **dead** `\normaldeviate`
  RNG branch (`x ≤ 0` unreachable) that additionally runs under a held borrow with a
  `Result` path (would double-borrow-panic, not longjmp); the other sites are
  `store_fmt_file` (Result) + `post_error_message` (absorbable terminal). Convertible,
  but multi-site `ErrorOutcome` threading — a separate follow-up.
- **`(B)` flip-gated (2): `open_log_file`, `finalize_dvi_file`.** Each has a
  threadable caller AND an un-threadable one *inside `into_history`'s terminal
  machinery* (`pre_error_message` / `close_files_and_terminate`), so converting
  forces the shared cleanup helper to return `Result` — a §6-flip concern.
- **Inline longjmps (2 classes):** `main_control get_preamble_token_in` (direct
  `fatal_error("interwoven alignment preambles") under the borrow, distinct from
  the folded deferred spine) + `ext.rs` native-font helpers (**13** direct
  `_tt_abort`, called deep from measurement/shipout/math/scan — no `Err` channel).

**Outcome (honest):** `(A)` came out **larger** than expected (8 converted +
`error()` convertible) and the true deferred `(B)` set is just **two** wrappers;
the structural remainder is the two inline classes. So the Rust longjmps still
gating the setjmp are a **small enumerable set** — `{open_log_file,
finalize_dvi_file}` + `error()` + `{main_control` align, `ext.rs` native-font ×13`}`
+ the dispatcher machinery reachable only through those live wrappers. This is the
precondition proof for Stage II.

**Metrics:** `unsafe fn` **263** (−0; deleted `trie_abort` is a `pub(crate) fn`,
not `unsafe fn`); `with_state` unchanged; warnings **37**. Validation: dashboard
**29/2/7/4** per commit; release rebuild; full corpus **637 passed / 2 diverged**
(known FP pair). Reverted a `cargo fmt` line-wrap churn in `format_io.rs` (as last
session). Commits `21409112`, `037b5dd8`, `5137505c`, `e24dd55b`.

**Next (`01d` §2/§6).** **Stage II (in-crate):** replace the ~19
`ttstub_input_seek`/`close` call sites with `ttbc_*`/`CoreBridgeState` + `Result`
threading, and fold the `xmalloc`/`xcalloc`/`xrealloc` `_tt_abort` jumps. Then the
remaining Rust blockers thread: `error()` (multi-site `ErrorOutcome`),
`open_log_file` + `finalize_dvi_file` (gated on the cleanup helpers), and the two
inline classes (`main_control` align variant, `ext.rs` native-font error channel).
Only once **no Rust path longjmps** AND Stage II neutralizes the bridge_core I/O
longjmps does the setjmp + owned-`EngineState` flip (§6) become possible.

## Idiom session 245 — 2026-07-01 — Execution: the main_control → EngineAbort fold (kill the main-control Rust longjmp spine)

**Type:** execution (continuation of the session-242 / 243–244 EngineAbort
lever). Autonomous: one all-or-nothing commit + a 31-agent adversarial verify.

**The fold (`01d` §5.5).** `main_control()` now returns `Result<(), EngineAbort>`:
`with_state_mut(main_control_in).map_err(|e| e.into_engine_abort())`, mapped
**post-borrow**. `tt_run_engine` catches it — `if let Err(a) = main_control() {
return a.into_history(); }` — **before** `final_cleanup`, so a fatal main-control
abort skips `final_cleanup` exactly as the old `e.fire()` longjmp-straight-to-the-
setjmp did (`01d` §5.5). This deletes the bulk of the remaining Rust longjmps.

**Mechanics.** Each of the ~31 leaf `*Abort`/outcome types in the main_control
tree gained a **non-unsafe** `into_engine_abort(self) -> EngineAbort`,
co-located with (and replacing) its former `fire()`/`finish()` dispatcher,
transcribing each terminal arm-for-arm: `overflow`→`Overflow`,
`confusion`→`Confusion`, `fatal_error`→`Fatal`, `pdf_error`→`PdfError`,
`_tt_abort(msg)`→`TtAbort(msg)`, `ErrorOutcome` Halt/Hundred→`HaltMessage`.
Message strings + numeric args preserved exactly; `into_history()` runs the
same print+cleanup the longjmp terminal did, so **format_buf surfacing fidelity
is automatic** (the class of bug the session-242 verify caught — `01d` §1).
Keeping them non-unsafe (internal `unsafe {}` blocks for the handful of side
effects) means the fold **adds zero unsafe fns**.

**Side-effecting leaves (`01d` §5.5), all preserved in order:** added
`crate::mem::main_memory_overflow_abort()` — the non-longjmp twin of
`fire_main_memory_overflow` (`runaway()` then the `"main memory size"`
Overflow). The MainMemory arms that ran `runaway()` route through it; the
**plain** `overflow("main memory size")` arms (SaMutation, ShowWhatever,
Shipout, PageBuild::MainMemoryOverflow, Hyphenate, LineBreak::Overflow,
NodeRecycle::Overflow) stay a **bare** `Overflow` with no `runaway` — the
distinction was checked per arm. `SccAbort::NoLegalEnd{open_log}` opens the log
before `Fatal`; NativeChar/LoadNativeFont `PoolOverflow` re-read
`(pool_size, init_pool_ptr)` via `with_state`; `ShipoutAbort::CannotOpenOutput`
rebuilds its message from `name_of_file()` via `snprintf` into a **leaked**
heap buffer so the pointer survives to `into_history`. `ErrorOutcome`'s
`post_error_message(false)` stays inside `into_history(HaltMessage)`, not
`into_engine_abort` (correct post-borrow timing).

**Deletions (`unsafe fn` 273 → 263).** `MainControlAbort::fire` + the 13 leaf
dispatchers whose sole caller was main_control became dead (Rust's *transitive*
dead-code closure flagged them all at once) and were deleted: free fns
`raise_save_abort`, `fire_page_build_abort`, `fire_line_break_outcome`; and the
`fire`/`finish` of PushNest/SaMutation/Unsave/InsertSrc/IssueMessage/ShowWhatever
+ NativeChar/ReadFontInfo/LoadNativeFont/NewCharacter/Hyphenate. (`fire_shipout_abort`,
`InputAbort::fire`, `SccAbort::fire`, the Open*/MakeTexString/Name fires **stay**
— still fired by their own non-main_control global wrappers, so not dead.) Two
now-unused `node_recycle_abort` imports removed to hold warnings at 37.

**Adversarial verification (Workflow, 31 independent skeptics, ultracode).** One
agent per leaf, each fetching the ORIGINAL `fire()`/`finish()` at HEAD
(`git show HEAD:…`), independently re-deriving the terminal + message strings +
numeric args + ordered side effects, and diffing against the new
`into_engine_abort` under the equivalence rules. Result: **31/31 EQUIVALENT,
128 arms, 0 divergent** (MainControlAbort 22 arms — every variant mapped, none
dropped; ShipoutAbort 10; PageBuild/LineBreakOutcome 9; SccAbort 8;
HyphenateAbort 7). The session-242 verify found a real `format_buf` bug in the
analogous class; this one found nothing.

**Metrics:** `unsafe fn` **273 → 263** (−10, the deleted dispatchers; the ~31
into_engine_aborts are all non-unsafe); `with_state` unchanged; warnings **37**.
Validation: dashboard **29/2/7/4** on the committed tree; release rebuild; full
corpus **637 passed / 2 diverged** (known FP pair `1905.00073v1`/`1905.00148v1`).
Also reverted a `cargo fmt` line-wrap churn in `format_io.rs` (a prior session's
over-width `return Err(...)` one-liners) to keep the commit focused. Commit
`997152e6`.

**Next (`01d` §2/§5).** The remaining Rust longjmp tail is the **non-main_control
scatter** (Stage-I residue): `maketexstring`/`start_input` + scattered
io/font/ext/pic aborts, each still `fire()`-ing via its own global wrapper (not
reached through main_control). Then **Stage II** (IN-CRATE): replace the ~19
`ttstub_input_seek`/`close` call sites with `ttbc_*`/`CoreBridgeState` + `Result`
threading, and fold the `xmalloc`/`xcalloc`/`xrealloc` `_tt_abort` jumps. Only
after both do the `setjmp` deletion + owned-`EngineState` flip (§6) become
possible. Don't start Stage II or the flip before the Stage-I residue is clear.

## Idiom session 243–244 — 2026-07-01 — Execution: format_io fully longjmp-free + EngineAbort terminal set; main_control gate disproved

**Type:** execution (continuation of session 242's EngineAbort lever).
Autonomous multi-commit run.

**Commits (4, dashboard 29/2/7/4 green each; `unsafe fn` 273, warnings 37):**
1. **`load_fmt_file` undump → `Result`** (`01d` §5.4). Fixes the longjmp-under-
   borrow hazard the session-242 verify workflow flagged: `load_fmt_file_in`
   holds one `&mut EngineState` borrow across the whole undump and `undump_bytes`
   `_tt_abort`'d on a short/corrupt read. Now `undump_bytes`/`undump_slice`/
   `undump_one`/`undump_int`/`undump_checked_i32`/`undump_upper_i32`/
   `undump_upper_u16` return `Result<_, EngineAbort>`; `load_fmt_file_in ->
   Result<bool, EngineAbort>` (78 undump `?` + 52 bad-format `Err` → `TtAbort`);
   `load_fmt_file -> Result<bool, EngineAbort>`; `tt_run_engine` maps
   `Ok(true)`→continue / `Ok(false)`→`return history()` / `Err(a)`→
   `a.into_history()`. **`format_io.rs` no longer references `_tt_abort`** — fully
   Rust-longjmp-free (dump + undump). TRIP validates (dumps AND undumps a format).
2. **`get_strings_started` → `Result`**: the last `tt_run_engine`-direct init
   fatal (`must increase pool_size`); `engine.rs` `_tt_abort` extern removed.
   `tt_run_engine`'s body + all its direct init helpers are now Result-based.
3. **EngineAbort + `Fatal`/`PdfError`**: `fatal_error`/`pdf_error` split like
   `overflow`/`confusion` (body → `into_history`, set `format_buf` to the same
   string the `_tt_abort` would).
4. **EngineAbort + `HaltMessage`**: the 6th terminal (`post_error_message(false)`
   + `_tt_abort(msg)`, the ErrorOutcome Halt/Hundred path). `ErrorOutcome::finish`
   routes its Halt/Hundred arms through it. **EngineAbort now covers all 6
   terminals** (Overflow/Confusion/Fatal/PdfError/TtAbort/HaltMessage) — the
   complete foundation for the `main_control` fold.
5. (docs)

**Headline finding — the `main_control` single-borrow gate is RESOLVED.** The
memory note `main-control-single-borrow-premature` (session 228) is stale:
`main_control()` is already `match with_state_mut(|st| main_control_in(st)) {
Err(e) => e.fire() }`, and `main_control_in(&mut EngineState) -> Result<(),
MainControlAbort>` holds **one** borrow over the whole loop — the entire
`main_control.rs` has only two `with_state` textual hits (this wrapper + a
comment), so no leaf remains to nest-panic. The Phase-C drain for main_control's
tree is complete. **So converting `main_control` to propagate `EngineAbort` to
`tt_run_engine` is UNBLOCKED** — the bulk of remaining longjmps, and the
unification step (1) from the original brief.

**Why it was deferred (not started this session).** `main_control()`'s `e.fire()`
is **post-borrow** (the `with_state_mut` already returned), so it is NOT a
longjmp-under-borrow hazard — converting it is critical-path progress toward the
final flip but NOT a correctness fix, and the flip is independently gated on
bridge_core Stage II. The fold is also genuinely intricate, not mechanical:
`MainControlAbort` aggregates ~21 leaf `*Abort` types; several have **pre-terminal
side-effects** (`SccAbort::NoLegalEnd` opens the log before `fatal_error`,
`node_recycle_abort::OverflowRunaway` calls `runaway()` first) and **deep
nesting** (`ErrorOutcome` threads through SccAbort/IssueMessage/ShowWhatever/
ReadFontInfo/NewCharacter/Vsplit). It is **all-or-nothing** (leaf
`into_engine_abort`s are dead code until `main_control` uses them, so no
incremental validation) and lands entirely on **fatal paths no test exercises**.
Big-banging that unsupervised for low marginal value was judged imprudent; it is
fully scoped in `01d` §5.5 as the next session's work (with adversarial verify).

**Validation:** dashboard 29/2/7/4 per commit; release rebuild; full corpus
**637 passed / 2 diverged** (known FP only) on the session 243–244 binary.

## Idiom session 242 — 2026-07-01 — Architecture + execution: EngineAbort — start killing the Rust longjmp spine (the real lever behind the wrapper/mod-raw endgame)

**Type:** architecture verification + execution (workstream 01 / Phase C — the
setjmp boundary). Drove the session-241 pivot: the ~98 `globals.rs` wrappers and
41 `mod raw` scalars drain only when the boundary fns can hold a real borrow,
which is blocked by the C `setjmp`/`longjmp`. The lever is **killing the Rust
longjmp**, not folding wrappers.

**Verified-architecture finding (don't take the pivot on faith).** Read
`xetex-engine-interface.c` + `bridge_core/support/support.c`. The setjmp at
`engine-interface.c:33` is the one-shot fatal bailout (no retry, no signals; the
`__terrible_…_musl_linker_hack` fn is a never-called linker hack). But it has
**three** longjmp sources, not one: Rust `_tt_abort` **plus** `bridge_core`
`ttstub_input_seek` (support.c:282) and `ttstub_input_close` (320), which longjmp
*from C* on internal I/O errors. So **deleting the setjmp** (the precondition for
the final RefCell/ownership flip) is gated not just on converting the Rust spine
but on neutralizing that input I/O longjmp — Stage II. **[Corrected post-session:
Stage II is IN-CRATE, not cross-crate — see `01d` §2.** The longjmp is only in the
C `ttstub_*` wrappers; the Rust `ttbc_input_seek`/`ttbc_input_close` they call
return errors as values, so engine_xetex calls those (or the `CoreBridgeState`
methods) directly + threads a `Result`. No shared-API change, no other-engine
impact.] Output I/O (`ttstub_output_write/close/flush`) does **not** longjmp,
which is why `store_fmt_file` could go fully Rust-longjmp-free. Also verified: the Rust
fatal spine is *already* almost entirely deferred-fire (37 `*Abort` enums + 11
`fire` fns, all reducing to 5 terminal causes — `overflow`/`confusion`/
`fatal_error`/`pdf_error`/`_tt_abort`); the inline longjmps that remain cluster
in the boundary fns themselves. Design doc: `01d-engine-abort-and-setjmp.md`.

**Execution (4 commits, dashboard 29/2/7/4 green each; `unsafe fn` 273, warnings
37 held throughout):**
1. **EngineAbort foundation** — unified terminal `EngineAbort {Overflow,
   Confusion, TtAbort}` in `errors.rs` with `into_history(self)->i32` (prints +
   cleans, **no** longjmp, returns `HISTORY_FATAL_ERROR`); `overflow`/`confusion`
   split to delegate then append `_tt_abort` (setjmp backstop for unconverted
   callers). Converted the two `tt_run_engine` init fatals (consistency check,
   primitive init) to `return …into_history()`. −2 longjmps.
2. **final_cleanup → `Result<(), EngineAbort>`** caught at `tt_run_engine`;
   `confusion`-on-dump-cleanup propagates instead of jumping. −1.
3. **store_fmt_file fully Rust-longjmp-free** — `dump_bytes`/`dump_slice`/… →
   `Result`, all 81 dump calls `?`-propagated (incl. the two `with_state` dump
   closures, which now return `Result`), the 4 flat fatals → `Err`. This also
   removes a **latent longjmp-under-borrow hazard** those session-241 dump folds
   carried (a short write inside the held `with_state` borrow would jump past the
   `RefCell` guard). −5.
4. **Surfaced-message fidelity fix** (found by the verify workflow, below) — add
   `_ttbc_set_error_message` to `bridge_core/support.c`; `into_history` writes
   `format_buf` to the exact string the longjmp would have, so the host's
   `EngineAbortedError` detail is unchanged.

**Adversarial verification (Workflow, 6 lenses, ultracode).** Independently
re-derived equivalence of each converted path against baseline `85bd7dc8`. 4
EQUIVALENT (overflow/confusion split byte-exact; final_cleanup cleanup-once
ordering; store_fmt_file normal-path byte sequence; store_fmt_file fatal arms). 1
**DIVERGENT → fixed**: the abort message string *is* host-surfaced (via
`with_global_lock`→`new_with_details`→`_ttbc_get_error_message`→`format_buf`), and
the `Result` path bypassed `_tt_abort` so left it empty — commit 4 fixes it. 1
DIVERGENT **pre-existing/out-of-scope**: `load_fmt_file_in` holds an `&mut`
borrow across the whole undump, and `undump_bytes` `_tt_abort`s on a corrupt
`.fmt` — a longjmp-under-borrow that predates this work (from the load-path fold)
and is the natural next conversion target.

**Metrics:** ~8 inline Rust longjmp sites → `Result` propagation to
`tt_run_engine`; `unsafe fn` **273** unchanged; `with_state` ~**296** (no
singleton folds — focused on the longjmp lever, not wrapper cosmetics per the
guidance); warnings **37**. Validation: `cargo fmt`/`check`; dashboard
**29/2/7/4** per commit (TRIP dumps+undumps a format, validating the
store_fmt_file normal path byte-for-byte); release rebuild; full corpus **637
passed / 2 diverged** (known FP only) on the final binary.

**Next:** (a) `load_fmt_file` → `Result` (fixes the verified undump
longjmp-under-borrow); (b) propagate the `!load_fmt_file()` early return at
`tt_run_engine:1295`; (c) the `main_control` cluster waits on the Phase-C
`with_state` drain before `main_control` can return `Result`. Fold each leaf
`*Abort` into `EngineAbort` only as its subsystem converts (not a 17-file
big-bang). Final setjmp deletion remains gated on the `bridge_core`
I/O-longjmp neutralization (Stage II) — which is **in-crate** (call `ttbc_*`/
`CoreBridgeState` directly; corrected, `01d` §2). See `01d` + memory notes
`setjmp-deletion-gated-on-bridge-core-io`, `engineabort-unification-progress`.

## Idiom session 241 — 2026-06-30 — Execution: globals.rs category-1 drain — fold the setjmp/IO boundary fns, delete 100 with_state wrappers

**Type:** execution (workstream 01 / Phase C — globals.rs accessor-wrapper
drain). Drove the session-240 "Next (b)": scope the 198 `with_state` accessor
wrappers in `globals.rs`, report the burn-down, then thread/delete the
deletable batch in small file-disjoint commits.

**Scoping finding (corrects the session-240 framing).** A deterministic
caller-classification (script, filtered for comment / local-closure-shadow /
`#[test]` / `.method` false positives) shows the session-240 hypothesis — "how
many of the 198 have *only* already-threaded callers (deletable now)" —
resolves to **exactly 0**. A threaded caller holds the top-level `borrow_mut()`,
so it **cannot** call a `with_state` wrapper without nest-panicking; no live
wrapper has an all-threaded caller set. The 198 decompose as:
- **4 dead** (zero callers).
- **9 test-only** — pinned solely by `#[test]` fns in `scaled_math.rs`
  (`arith_error`/`tex_remainder`/`random_entry`/…); deletable only by rewriting
  test setup.
- **22 leaf-mixed** — pinned by the error-panic trio (`confusion`/`overflow`/
  `fatal_error`, called from hundreds of unthreaded sites), the deferred
  abort-`fire` helpers (run *after* the borrow drops, by design), and a few
  file/name leaves — but every one is **also** boundary-pinned, so none dies
  from threading a leaf alone.
- **163 boundary-only** — pinned by the setjmp/IO boundary fns (`tt_run_engine`
  79, `store_fmt_file` 43, `final_cleanup` 10, `close_files_and_terminate` /
  `tt_cleanup` / `tt_xetex_set_int_variable` …) that structurally **cannot**
  take `&mut EngineState` (they straddle the C setjmp/longjmp boundary and
  bootstrap the state).

So the wrappers are **not a free repoint**. They drain only by **folding their
boundary callers into held borrows** (the session-232 island pattern: replace N
per-read wrapper borrows with one `with_state(_mut)` reading `st.field`
directly, deleting any wrapper whose last caller is thereby repointed). That
fold is the *same* structural setjmp wall the `mod raw` fold waits on — i.e. the
category-1 drain is **gated on the boundary**, not independent of it. (The data
already lives in `EngineState`, but "die as legacy callers thread" requires
folding the setjmp boundary; that is the hard half.)

**Commits (11), file-disjoint per boundary fn (+ globals.rs deletes),
dashboard-validated each:**
1. delete 4 dead wrappers (`set_lo_mem_max`/`mem_gr`/`help_ptr`/`rule_dp`).
2. `tt_xetex_set_int_variable` → one borrow over the dispatch (−5 config setters).
3. `close_files_and_terminate` → fold write-stream/log/transcript reads
   (−4: `write_open`/`write_file`/`log_file`/`texmf_log_name`).
4. `init_io` → one input-buffer-init borrow (−3: `last`/`set_last`/`set_input_file`).
5. `tt_cleanup` → repoint the font-release + input-close loops
   (−4: `font_layout_engine_ptr`/`set_font_layout_engine_ptr`/`in_open`/`input_file`).
6. `get_strings_started` (−2: `set_str_start`/`string_vacancies`).
7. `final_cleanup` → hoist scalars + batch the `\if`-node scanner writes
   (−10: `input_ptr`/`open_parens`/`cur_level`/`cond_ptr`/`cur_if`/`if_line`/
   `mem_b16s0`/`set_cond_ptr`/`set_cur_if`/`set_if_line`).
8. `store_fmt_file` → fold every dump region (mem free-list, eqtb/hash
   run-length, prim, font table, hyph/trie): snapshot immutable scalars once,
   track mutated ones (var_used/dyn_used/cs_count/hyph_next) as locals written
   back once, read each pair under one borrow via closures; inline
   `pack_buffered_name`'s `set_name_of_file` (−38, incl. `hash_high`).
9. `tt_run_engine` nullfont + trie-table init blocks → single borrows
   (`maketexstring` computed before the borrow; shared `set_font_mapping_ptr`
   write inlined under it) (−30 setters).

**Metrics:** globals.rs `with_state` accessor wrappers **198 → 98** (−100);
total `with_state` occurrences **352 → 296** (−56, net drop despite the ~1:1
inlines, from the folded multi-read regions); `unsafe fn` **273** unchanged
(pure behavior-preserving); cargo warnings **37** held.

**Validation:** `cargo fmt`; `cargo check` 37 warnings; dashboard **29/2/7/4**
per commit (TRIP exercises `\dump`+format-undump and `\if`-incomplete recovery —
the high-risk paths these touch); release rebuild; full corpus **637 passed / 2
diverged / 0 missing-golden** (only known FP `1905.00073v1`/`1905.00148v1`).

**Next:** the remaining **98** wrappers are (a) ~49 `tt_run_engine`-exclusive
**scattered singleton** setters — each a 1:1 `set_X(v)`→`with_state_mut(|s| s.f =
v)` swap that deletes the wrapper but leaves `with_state` flat, interspersed
with FFI / format-undump branches (low ROI, do only if pushing the wrapper
count); (b) the **multi-boundary-pinned shared** wrappers (`selector`/
`interaction`/`history`/`set_eqtb_b32s1`/`set_selector`/…) that die only when
*all* their boundary callers fold; (c) the error-panic trio + 9 test-only,
needing different treatment (thread `confusion`/`overflow`/`fatal_error` — a
hundreds-of-sites ripple — or rewrite the `scaled_math` tests). Honest
end-state: the last wrappers **and** the 41 `mod raw` scalars both fall only
when the setjmp boundary itself is restructured to own `EngineState` directly
(then `with_state` / `thread_local! RefCell` can be deleted); this session
removed the cheap half of the boundary-fold work.

## Idiom session 240 — 2026-06-30 — Execution: typed field-read sweep (11 commits) + node accessor surface; threading-frontier writeup

**Type:** execution (workstream 01 / typing — adoption + first signature
propagation). One read-only recon workflow (9 parallel agents) classified every
raw `st.mem.bXXsY(ptr+N)` site across the 9 hot files into GREEN (typeable now) /
YELLOW (needs a small new accessor) / RED (bucket-D). Implemented sequentially,
file-disjoint, dashboard-validated per commit. Pure behavior-preserving:
`with_state` **285** and `unsafe fn` **273** unchanged.

**Commits (11):**
1. scan glue-spec arithmetic (`\advance`/order/ref_count/MU-coerce via
   `GlueSpecNode`) + `set_math_char` noad type write.
2. linebreak native-word text/glyph pointers (`native_text_ptr`/
   `native_glyph_info_ptr`; new `set_native_length`; deleted dead
   `NATIVE_NODE_SIZE`/`_NN`).
3. packaging `append_to_vlist` baselineskip width.
4. main_control `do_register_command` `\advance`/`\multiply`/`\divide` +
   `fin_align` tabskip glue-spec.
5. **node.rs accessor surface** (additive, `mod node` is `#[allow(dead_code)]`):
   InsNode `float_cost`/`depth`/`height`(+set), UnsetNode `glue_stretch`/
   `glue_shrink`/`glue_order`/`glue_sign`, SaNode `from_ptr`/`sa_ptr`,
   WhatsitNode `set_native_font`/`set_native_glyph` + `write_stream`/`open_name`/
   `open_area`/`open_ext`.
6. math AnyBox glue setters (`init_math`), `app_display` p_orig/b_copy classify,
   native-glyph construction (`new_stack_glyph`/`make_op`/`mlist_to_hlist`).
7. output node-display reads (unset glue/ins/open/write) + `print_write_whatsit_in`
   typed param (`NodePtr<WhatsitNode>`); `short_display` subtype reads.
8. pagebuilder insertion height/depth/float_cost + eTeX SaNode marks
   (`child(1)`/`child(2)`).
9. expand sparse-array box/mark reads via SaNode (`sa_ptr` + collapsed `child(t)`).
10. main_control `begin_insert_or_adjust` Ins/Adjust + `make_mark` fresh-node typing.
11. shipout write/open whatsit fields.

**Metrics:** raw `st.mem.bXXsY(ptr+N)` node-field accesses ~**366 → 308** (−58);
`with_state` **285** / `unsafe fn` **273** unchanged; cargo warnings **37** held.

**Validation:** `cargo fmt`; `cargo check` 37 warnings; dashboard **29/2/7/4** per
commit; release rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**
(only known FP `1905.00073v1`/`1905.00148v1`).

**Frontier finding (the headline for whoever wants to "finish" this).** The engine
is *functionally* fully threaded — every algorithm runs under `&mut EngineState`.
The residual ~285 `with_state` is **structural, not backlog**, and splits in two
(verified against `state.rs:1455-1466` + `globals.rs`):
- **198 globals.rs `with_state` accessor wrappers** (`arith_error()` →
  `with_state(|s| s.errors.…)`): the data **already lives in `EngineState`**; each
  wrapper dies once its remaining legacy callers are threaded to read `st.X`
  directly. Broad/lateral, finite, low-ROI — **NOT blocked**.
- **41 `mod raw` `static mut` hot scalars** (`cur_cmd`/`cur_chr`/`cur_list`/
  `cur_input`/`temp_ptr`/`main_*`/lig locals): read **borrow-free**
  (`fn cur_cmd() -> u8 { unsafe { raw::cur_cmd } }`, NOT via `with_state`) so a
  threaded core can touch them while holding the top-level `borrow_mut()`. They
  must stay raw until **no `with_state` path touches them** — otherwise a
  `with_state` leaf reached under the live `&mut` nest-panics (RefCell
  double-borrow, `state.rs:1461`; this is the session-228 failure). **Sequencing
  constraint, NOT a perf wall** (after inlining `st.input.cur_cmd` == the raw
  load; the RefCell cost is paid once at the top). They are the LAST domino: fold
  into `EngineState` only after the category-1 wrappers + setjmp-I/O leaves drain,
  then `with_state`/`thread_local! RefCell` can be deleted entirely.

**Next:** two tracks. (a) **Typing tail** — ~15-30 clean sites left
(math `make_math_accent` glyphs, `box_end`/`handle_right_brace` noad, shipout
PIC/edge which need new span/pic/edge families). (b) **globals.rs category-1
drain** (the path that unblocks the final `mod raw` fold) — scope first: of the
198 wrappers, how many have *only* already-threaded callers (deletable today by
repointing to `st.X`) vs how many are pinned by a still-unthreaded caller; report
that burn-down number, then thread the deletable batch in file-disjoint commits.

## Idiom session 239 — 2026-06-30 — Execution: typed-propagation sweep (is_null, alloc-twins, FractionAssembly/fin_align AnyBox)

**Type:** execution (workstream 01 / typing — propagation). Five file-disjoint,
dashboard-validated commits, preceded by one read-only recon workflow (8 parallel
agents) that classified every `.idx().0`/`from_idx_unchecked`/null-check site
across the 7 hot files into propagatable vs justified-i32-boundary. Pure
behavior-preserving typing: `with_state` **352** and `unsafe fn` **273**
unchanged.

**Commits:**
1. **is_null sweep** — 26 raw `<accessor>.0 == TEX_NULL`/`.0 != TEX_NULL` null
   checks on MemPtr chains (list_ptr/pre_break/post_break/child/break_node) →
   `.is_null()`/`!…is_null()` across linebreak (12), packaging (13), math (1).
2. **shipout movement loop** — `while q.link(mem).0 != p.idx().0` →
   `q.link(mem) != MemPtr::from_idx(p.idx())` (both DVI y/z reuse loops).
3. **math kern/fraction typing** — `overbar` takes `AnyBox` and builds via the
   typed `alloc_kern_node` (not `new_kern` + `from_idx_unchecked`);
   `make_under_prepare`/`make_ord` adopt `alloc_kern_node`; `FractionAssembly.x/z`
   → `AnyBox` so `make_fraction` reads them directly. −4 `from_idx_unchecked`,
   −2 `classify_box`.
4. **fin_align_in** — the §836-840 packed template box classified once
   (hpack/vpack already return `AnyBox`); 15 raw `mem.bXXsY(p+N)` reads →
   typed `AnyBox` accessors (width/height/depth/glue_sign/glue_order/glue_set/
   list_ptr); −2 `.idx().0`. **TRIP caught an initial depth/height +2/+3 offset
   swap during validation** (depth is +2, height +3) — fixed and re-validated.
5. **typed null-box allocators** — added `alloc_null_box`/`alloc_null_vbox`
   (typed HLIST/VLIST twins of `new_null_box`, confining `from_idx_unchecked` to
   one audited site each, like the existing `alloc_kern_node`); adopted at
   `char_box`/`make_fraction`/`native_glyph_vbox_start`/`build_ext_delimiter`
   (−2 `classify_box`, −2 inline `from_idx_unchecked`).

**Metrics:** `.idx().0` **125 → 120**; `.0 [!=]= TEX_NULL` **24 → 0**;
`from_idx_unchecked` **47 → 43**; math.rs `classify_box` **~25 → 22**;
`with_state` **352** / `unsafe fn` **273** unchanged; cargo warnings **37** held.

**Key finding (strategy — recorded in [[propagate-typed-returns-not-idx0]] /
[[typing-frontier-is-bucket-d-residue]]):** the recon over-flagged flipping the
`new_*` constructors (new_kern/new_glue/new_noad/new_null_box) to
`Option<NodePtr<K>>`. **Descoped**: a global return-type flip sprays `.idx().0`
at the i32 link / `cur_list_tail` / state stores (the consuming side stays raw
`mod raw`) and forces awkward `map_or(ALLOC_FAILED,…)` at callers that share an
i32 binding with raw reads (math.rs 4316/4741) or other i32 constructors — all
to remove a handful of `from_idx_unchecked`. The clean alternative used here is
the **typed-twin allocator** (`alloc_kern_node`/`alloc_null_box`/
`alloc_null_vbox`): confine `from_idx_unchecked` to one audited allocator and let
the *re-classifying* callers adopt the typed handle, leaving the i32-link-store
callers on `new_*`. Matches session-238's drained-pool assessment.

**Validation:** `cargo fmt`; `cargo check` 37 warnings; dashboard **29/2/7/4**
per commit; release rebuild; full corpus **637 passed / 2 diverged / 0
missing-golden** (only the known FP `1905.00073v1`/`1905.00148v1`),
doc-count == baseline, no NOXDV/NOLOG.

**Next:** the recon-identified clean typing pool is drained — the remaining
`.idx().0` (math 56, mem 17, main_control 12, linebreak 12, packaging 11) are
justified i32 boundaries (noad packed-record nucleus stores, i32-list-head
callees `free_node`/`hpack_emit`/`show_box`, link stores into raw
`cur_list_tail`/cursors per [[mod-raw-is-intentional-not-debt]]). Further
typed-twin adoptions (`alloc_noad`/`alloc_glue`) are marginal (1–2 re-classify
sites each). The larger remaining vein is **threading leaves to shrink the 352
`with_state` sites** (session-238's call) — now mostly setjmp-anchored IO/init
(io/output/format_io/texmfmp/engine) + the bucket-D measurer interleavings in
`make_op`.

## Idiom session 238 — 2026-06-30 — Execution: link-store idiom + math box/glyph-creator return propagation

**Type:** execution (workstream 01 / typing — *propagation*). Mechanical
typing session, one commit, dashboard + full-corpus validated. Pure
behavior-preserving typing: `with_state` **352** and `unsafe fn` **273**
unchanged.

**Code:**
- **Link-store idiom (25 sites).** Every `MemPtr::from_raw(X.idx().0)` →
  `MemPtr::from_idx(X.idx())` across math/linebreak/pagebuilder — the
  documented store-a-typed-handle-into-a-pointer-field form
  (`propagate-typed-returns-not-idx0`). `MemPtr::from_idx` 7 → 32.
- **`char_box` chain → `AnyBox`.** `char_box`/`char_box_tfm` now return the
  `AnyBox` they already classify internally (`bb`) instead of the raw index,
  and `stack_into_box` takes `AnyBox` for its stacked char box. Dropped the
  re-classify in `char_box_in` and the one inside `stack_into_box`; the
  OpenType-assembly `char_box_tfm` callers feed the handle straight in with
  no new `.idx().0`. −2 `classify_box_or_panic`.
- **`make_fraction` chain → `AnyBox`.** `make_fraction` returns its `vb`
  handle; `make_fraction_link_delimiters` takes `AnyBox`×3 and stores links
  via `set_link(MemPtr::from_idx(..))`. −2 `.idx().0` at the call site.
- **`build_opentype_assembly_in` typed writes.** Added `set_glue_sign`/
  `set_glue_order`/`set_glue_set` delegators on `AnyBox`; the resolved-
  dimension block now writes glue_sign/order/set + width/depth through the
  once-classified `b_box`, and the function returns it (−1
  `classify_box_or_panic`, 5 raw `b + N` writes typed).
- **Glyph-node creator → `NodePtr<WhatsitNode>`.** `new_stack_glyph` returns
  the typed glyph whatsit handle; `stack_glyph_into_box` /
  `native_glyph_vbox_{start,finish}` carry it (`WhatsitNode: HasNodeHeader`,
  so `set_link`/`idx` are available) and drop their `classify_whatsit`
  re-classification (the `measure_native_glyph` raw-pointer base now derives
  from `p.idx().index()`). −2 `classify_whatsit`.

**Metrics:** `.idx().0` **150 → 125**; math.rs `classify_box_or_panic`
**20 → 17**; math.rs `classify_whatsit` **5 → 3**; `MemPtr::from_idx`
**7 → 32**.

**Validation:** `cargo fmt`; `cargo check` **37** warnings (held); dashboard
**29/2/7/4**; release rebuild; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (the glyph/native
slice is the OpenType-math path — corpus is the key guard).

**Next:** the clean propagation pool in math.rs is largely drained — the
remaining `.idx().0`/`classify_box_or_panic` are justified `i32` boundaries
(noad packed-record words like the overloaded nucleus_type/new_hlist word,
`i32`-list-head callees `hpack_emit_in`/`overbar`/`rebox`/`free_node`, the
after_math `a`/`b` nullable re-typed locals, and `cur_box`/`cur_list_tail`
raw hot-path state per `mod-raw-is-intentional-not-debt`). Threading leaves
to shrink the 352 `with_state` sites is the larger remaining vein.

## Idiom session 237 — 2026-06-30 — Execution: type the box-creator RETURNS (packaging core + math delimiter/char builders)

**Type:** execution (workstream 01 / typing — *propagation*). Big mechanical
session, three commits, each dashboard + full-corpus validated. Mid-session
review feedback corrected the approach and is now in memory
([[propagate-typed-returns-not-idx0]]): `from_idx_unchecked(MemIdx::new(x))` is
an anti-pattern (re-deriving a handle that already exists) and blanket `.idx().0`
at consumers defeats propagation — `.idx().0` is kept *only* at justified `i32`
boundaries (i32 state fields, child/link stores via `MemPtr::from_idx`, nodes
about to be re-typed, `i32`-only callees).

**Commit 1 (`cfc8013b`) — packaging core returns `AnyBox`.**
`hpack_emit_in`/`vpackage_emit_in` now return `AnyBox`. `PackOutcome` carries the
classified `NodePtr` handle that `hpack_in`/`vpackage_in` already build (field
`r_box: AnyBox`), so the emit boundary returns it verbatim — no
`from_idx_unchecked` re-wrap. The over/underfull diag printers + common-endings
take `AnyBox` (dropping their internal classify). ~38 callers across
math/main_control/linebreak/pagebuilder updated: re-classifications collapse to
the carried handle; consumers use typed accessors; `.idx().0` only at i32
boundaries. Added `MemPtr::from_idx(MemIdx)` for the handle→pointer-field store
idiom. `post_line_break_suffix` now takes `AnyBox`. math.rs
`classify_box_or_panic` 40→32.

**Commit 2 (`799c5984`) — math delimiter/char builders return `AnyBox`.**
`char_box_in`, `native_glyph_vbox_in`, `build_ext_delimiter_box`,
`build_opentype_assembly_in` now return `AnyBox` (classify once at their
hpack/vpack/arena creation site). `var_delimiter_in`'s internal `b` is typed
end-to-end so its final classify is gone; `make_math_accent`'s `y` carries the
`char_box_in` handle (7 re-classifications removed). 32→25.

**Commit 3 — collapse same-box re-classifications.** Hoisted one box handle in
`make_op` (`v`) and `make_math_accent` (`x_box` once after the non-null guard;
the reclean block reads height off the `clean_box_in` handle), and removed the
`let y_box = y;` rebind artifacts. 25→20.

**Validation:** `cargo fmt`; `cargo check` **37** warnings (held) per commit;
dashboard **29/2/7/4** per commit; release rebuild; full corpus **637 passed / 2
diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`, per
commit. `with_state`/`unsafe fn` unchanged (pure typing).

**Next:** typed state fields (`st.scanner.just_box`/`cur_box`) would retire the
remaining main_control/linebreak `.idx().0` at field stores. The 20 residual
math.rs `classify_box_or_panic` are genuine creation boundaries (clean_box
single-box, rebox/char_box/build_* returns, opentype-assembly measure) or
nullable/re-typed raw locals (`after_math` `a`/`b`, alignment `just_box`/`j`) —
leave them.

## Idiom session 236 — 2026-06-30 — Execution: typed field-read sweep (8 files) + first AnyBox return-propagation

**Type:** execution (workstream 01 / typing). Very large mechanical session, then
a review-driven pivot to handle *propagation*. One read-only recon workflow (8
parallel agents, one per candidate file) produced a per-site classification of
every raw `mem.bXXsY(expr+N)` into typeable-now slices vs bucket-D residue;
implementation stayed sequential + per-commit dashboard-validated, full corpus
verified at the field-typing milestone and again after the propagation commit.

**Code (12 commits).** Field-read typing adopting the existing typed surface
(`classify_*`/`NodePtr<K>`/`AnyBox`) across **scan, linebreak, output, expand,
packaging, shipout, and math.rs (5 batches)**: ~**117** threaded raw `+N` reads
(grep `mem.b(32|16)s[0-3](x+N)`: **329 → 212** over the 8 files) now go through
typed handles classified once at a boundary. Polymorphic list cursors were
converted to classify-once-per-node `match mem.classify(p)` dispatch
(`short_display_in`, the `init_math`/`build_opentype_assembly` width-measure
loops) instead of raw discriminant reads. Then a **return-type propagation** (two commits):
`clean_box_in`/`rebox_in`/`var_delimiter_in` now return `AnyBox` (classifying
once at their hpack/arena creation boundary), so `make_over`/`make_under`/
`make_radical`/`make_fraction`/`make_op` carry the handle and **11 redundant
re-classifications were removed** (9 `classify_box_or_panic` for the clean_box/
rebox layer + 2 `classify_box().unwrap().unwrap()` for the var_delimiter `y` in
make_radical); `.idx().0` survives only at arena-field stores and still-`i32`
callees (hpack/overbar/free_node/FractionAssembly/make_radical_prepare_delimiter/
set_nucleus_type). `rebox_in`/`var_delimiter_in` take their box context so the
re-box reassignments stay typed end-to-end.

**Review feedback adopted (recorded in memory):** (1) carry typed `NodePtr<K>`
handles, never the temporary `*_node_*_in` i32 bundlers
([[carry-typed-handles-not-i32-bundlers]]); (2) classify the *cursor* in
polymorphic walkers (typed `match mem.classify(p)`), don't read raw discriminants;
(3) null checks use `MemPtr::is_null()`, never `ptr.0 != TEX_NULL`; (4) **the
endgame is propagation** — classification happens once (creation/match boundary)
and the typed handle flows through return types/params/state fields, so
`classify_box_or_panic`/`assume_noad` eventually vanish.

**Validation:** `cargo fmt`; `cargo check` warnings **37** (held) per commit;
dashboard **29/2/7/4** per commit; release rebuild; full corpus **637 passed / 2
diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`, at the
field-typing milestone and after the propagation commit. `with_state`/`unsafe fn`
unchanged (pure behavior-preserving typing). Remaining `classify_box_or_panic` in
math.rs: **40**, all on genuinely-`i32` creator results not yet typed.

**Next (continue the propagation bottom-up — this is the headline direction):**
type the foundational box creators' returns so the remaining re-classifications
collapse — `new_null_box`, `var_delimiter_in`, `char_box`/`char_box_in`, then the
packaging core `hpack_emit_in`/`vpackage_emit_in` (cross-file, corpus-sensitive —
do as its own careful bottom-up sequence). Then typed *state fields*
(`st.scanner.just_box`/`cur_box`/…). `assume_noad` is a true polymorphic-dispatch
helper but its callers can often receive `NodePtr<Noad>` directly. Still-pending
field slice: `app_display_in` (the display L/R glue-surgery + 3×-reassigned `b`).

## Idiom session 235 — 2026-06-29 — Execution: typed dispatch + box-header reads (linebreak/shipout)

**Type:** execution (workstream 01 / typing). Large mechanical session, four
file-disjoint typed slices, one consolidated doc update. One Explore subagent did
a read-only per-site family manifest for `linebreak.rs` (which `b16s1 as i32`
sites are real type-tests vs. char-node font reads vs. delicate DELTA-walks);
implementation stayed sequential + per-commit validated.

**Code (4 commits):**
- **`linebreak::extract_break_node`** — the post-line-break GLUE/DISC/KERN/MATH
  `else if mem.b16s1(q) as i32 == …` chain became typed `AnyNode` classify
  matches (`if let AnyNode::Glue(q_glue) = mem.classify(…)`), one classified
  handle per arm; four raw discriminant reads + unchecked wrappers gone. Once
  per line break (not a per-char hot loop).
- **`shipout::ship_out`** — classify the shipped page box once at the huge-page
  guard; width/height/depth and the vlist/hlist dispatch now read through
  `AnyBox` (~12 raw `mem.b32s1(p + N)` reads + the `b16s1(p)` type test gone).
- **`shipout::hlist_out_in`/`vlist_out_in`** — added `AnyBox`
  `glue_sign`/`glue_order`/`glue_set`/`subtype`/`set_subtype` delegators, then
  classified `this_box` once at the top of each; glue-set/order/sign, list_ptr,
  width/height/depth, and the LR-direction subtype (DLIST/REVERSED/upwards)
  reads+writes now go through the typed box handle (~20 raw `this_box + N`
  accesses gone from the DVI output core).

**Deliberately left raw (bucket D, prior-session deferrals confirmed):** the
`output_hlist_glue_in`/`output_vlist_glue_in` glue-spec paths (the spec `g` is
reallocated + node `p` retyped mid-function), the polymorphic leader_box
(rule-or-box), the native-word ligature/kern reconstruction loop, math noads,
and the linebreak `try_break` inner-loop dispatch + active-list DELTA-walks
(hot path; already in acceptable typed-dispatch form). packaging.rs residue
(EDGE/SYNCTEX/PAGE_INS) still waits for proper typed surfaces (session 234 Next).

**Validation:** `cargo fmt`; `cargo check` warnings **37** (held); dashboard
**29/2/7/4** per commit; release rebuild; full corpus **637 passed / 2 diverged /
0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count ==
baseline, **no NOXDV/NOLOG** (the box-classify `.expect()` would panic→NOXDV on
any misclassification — the session-187 guard is the key check here). Metrics:
`with_state` **285** (unchanged — pure typing); `unsafe fn` **273 → 272**;
shipout raw `+N` field reads **80 → 59** (total `bXXsY` **275 → 247**); linebreak
untyped `b16s1 as i32` type-tests **42 → 38**.

**Next:** the clean box/dispatch-typing veins in shipout/linebreak are drained.
Remaining typing is bucket-D residue (glue-spec realloc paths, polymorphic
leader_box, native-word loops, noads) or hot-loop dispatch best left in its
current typed-dispatch form. math.rs (128 raw `+N`) is the largest pool but is
noad-packed-record heavy (rank 8) — type its box/glue/kern sub-reads
opportunistically, leave noads on the worklist.

## Idiom session 234 — 2026-06-29 — Execution: primitive threading + typed shipout/page lifecycle

**Type:** execution (workstream 01 / fused threading+typing). Large mechanical
session with three parallel, file-disjoint implementation slices; documentation
consolidated here.

**Code:** threaded primitive registration/initialization through
`&mut EngineState`, deferring string/hash/buffer aborts until the anchor borrow
drops. Deleted the last two `_global` wrappers, their unused C exports/header
prototypes, and seven dead globals accessors. Retyped both shipout list walkers
and the pagebuilder build/fire lifecycle around one `AnyNode` classification per
ordinary node, existing typed box/rule/whatsit/glue/kern/penalty/ins/mark
surfaces, and exact `RuleDimen::raw()` handling. Raw char nodes, LR/PAGE_INS
records, sentinels, token refcounts, and unsupported EDGE fields remain raw.
Commits: `4b21f301`, `a0b3297e`, `d818d1b7`.

**Validation:** `cargo fmt --all -- --check`; `cargo check` (warnings **37**);
dashboard **29/2/7/4**; release rebuild; full corpus **637 passed / 2 diverged /
0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Metrics:
`with_state` calls **293 → 285**; `unsafe fn` **274 → 273**; `_global` wrappers
**2 → 0**; Rust C exports **13 → 11**; bare `p/q/r: i32` params **266 → 261**;
raw mem-field calls shipout **285 → 224**, pagebuilder **152 → 101**.

**Next:** continue typed list processing in `packaging.rs` or `linebreak.rs`.
The remaining PAGE_INS and EDGE fields should wait for proper typed family
surfaces rather than gaining local raw-to-raw wrappers.

## Idiom session 233 — 2026-06-29 — Execution: full main-control borrow + typed node display + bridge drain

**Type:** execution (workstream 01 / fused threading+typing). Large mechanical
session; three subagents performed read-only frontier audits and one implemented
the independent typed display slice. Documentation consolidated here.

**Code:**
- Converted the complete top `main_control` `big_switch`/`reswitch` loop to
  `main_control_in(&mut EngineState) -> Result<(), MainControlAbort>`. It now
  calls the scanner, tracing, character loop, and all command dispatchers under
  one borrow. The public anchor wrapper fires deferred aborts only after the
  borrow drops; the four short-borrow `mc_*` trampolines are gone.
- Converted `output::show_node_list_in` to classify each ordinary lo-mem node
  once and use `AnyNode`/`AnyBox` plus existing per-family accessors. Unknown
  type output, bounds checks, nullable recursive lists, and hi-mem character
  handling remain unchanged; genuinely untyped specialized fields remain raw.
- Drained eight `_global` wrappers from string-pool startup, format dumping,
  pseudo-file cleanup, and final INITEX cleanup. The direct borrow islands keep
  cold `overflow`/`confusion` firing outside the borrow. Removed the wrappers
  and six now-dead globals accessors. Only the stale exported hash lookup shims
  remain.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` (warnings
**37**); dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full
corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Metrics: `with_state` text matches
**374 → 293**; `unsafe fn` **276 → 274**; `_global` wrappers **10 → 2**.

**Next:** thread `primitive`/`initialize_primitives` over direct
strings/hash/eqtb/input state, then delete the zero-C-caller `id_lookup` and
`prim_lookup` export shims plus stale header prototypes. The next large typed
vein is shipout's `hlist_out_in`/`vlist_out_in` traversal.

## Idiom session 232 — 2026-06-28 — Execution: collapse init/cleanup + format-dump bridge islands; delete dead string wrappers

**Type:** execution (workstream 01). Large mechanical session, one consolidated
doc update, one commit. No new threading of reachable main-control leaves was
needed — `main_control.rs` is effectively fully threaded (only the session-228
`mc_*` dispatch wrappers remain), so this session drained the explicitly-flagged
init/cleanup/format-IO bridge mass.

**Code:**
- **engine.rs anchors.** `tt_cleanup`'s 8 adjacent arena-release `with_state_mut`
  islands collapsed into one borrow; `tt_run_engine`'s per-run allocation block
  (9 islands) collapsed into one borrow with all `*_size + 1` sizes precomputed
  from the `tre_*` locals; the initex mem+eqtb-sizing islands and the
  `trie_not_ready` font-array islands each merged pairwise. These are top-level
  setjmp anchors (can't take `&mut EngineState`, §2), so the legitimate move is
  minimizing borrow count, not threading the signature.
- **format_io.rs dump side.** The ~14-island font-table dump region folded into
  a single `with_state` borrow (the interleaved `dump_int`/`dump_slice` are file
  FFI and `font_ptr()`/`fmp` are raw, so none re-enter `with_state`; dump order
  preserved byte-for-byte). The six trie/hyf base-pointer snapshots collapsed to
  one tuple snapshot. The undump side (`load_fmt_file_in`) was already threaded.
- **Dead bridge deletion.** Removed 8 zero-caller `_global` wrappers:
  `length`/`append_str`/`str_eq_buf`/`str_eq_str`/`search_string`/
  `slow_make_string`/`flush_str` (string_pool.rs) + `cur_length` (hash.rs). Their
  threaded cores keep direct callers, so nothing was orphaned. The remaining 10
  `_global` wrappers are all legitimately reached from the run-once anchors
  (`final_cleanup`/`store_fmt_file`/init) and stay until the anchors are
  restructured (out of scope).

**Validation:** `cargo fmt`; `cargo check` warnings **38** (one dead-code warning
gone with `cur_length_global`); dashboard **29/2/7/4** (TRIP exercises format
dump/undump directly — the key guard); release rebuild; full corpus **637 passed
/ 2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`.
Metrics: `with_state` text matches **415 → 374**; `unsafe fn` **278 → 276**;
`_global` wrappers **18 → 10**.

**Next:** the clean island-collapse / dead-wrapper veins are now drained. Further
`with_state` reduction requires threading the scattered legacy callers of the 10
remaining anchor-reached `_global` wrappers (broad, lateral on the count) or
restructuring the setjmp boundary itself. The output/io/texmfmp/string thin
`_global` print/string wrappers drain only as their legacy callers thread.

## Idiom session 231 — 2026-06-27 — Execution: initializer threading + typed node leaves

**Type:** execution (workstream 01 / fused threading+typing). Large mechanical
session with one consolidated documentation update; three subagents handled
independent recon/implementation slices.

**Code:** `initialize_more_variables_in` and
`initialize_more_initex_variables_in` now initialize Rust-owned state directly
under one borrow apiece; 100+ dead global setters/readers were removed.
Shipout initialization now uses one borrow, and the native XDV whatsit branch
classifies once to `NodePtr<WhatsitNode>` through
`make_xdv_glyph_array_data_in`. Linebreak `goto_found` now carries typed
ligature handles and allocates/updates discretionary nodes through
`NodePtr<DiscNode>`.

**Validation:** commit `db17878e`; `cargo fmt`; `cargo check` warnings **39**;
dashboard **29/2/7/4** before and after commit; release rebuild; full corpus
**637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1`/`1905.00148v1`. Metrics: `with_state` **518 → 415**;
`unsafe fn` **281 → 278**.

**Next:** continue draining bridge leaves before another `main_control_in`
attempt. The remaining `tt_run_engine` initialization/cleanup islands are a
large mechanical batch; string/hash/output wrappers and reachable
main-control leaves are the other main veins.

## Idiom session 230 — 2026-06-26 — Execution: thread input encoding + small typed-node cleanups

**Type:** execution (workstream 01 / fused threading+typing). Larger
mechanical session, minimal docs; used two explorer subagents for target
scouting, edits stayed local.

**Code:** threaded the input-encoding path reached from `do_extension_in` and
`u_open_in_in`: `ext::get_encoding_mode_and_info_in(&mut EngineState, ...)`
now prints diagnostics through `_in` output cores and creates ICU mapping
strings through `maketexstring_in`; `io::set_input_file_encoding_in(&mut
EngineState, ...)` now uses `gettexstring_in` and threaded diagnostics. The
legacy `set_input_file_encoding` remains as a thin wrapper, while the dead
`call_set_input_file_encoding`, `get_encoding_mode_and_info`, old
`makeXDVGlyphArrayData`, and dead global `begin_diagnostic`/`end_diagnostic`
wrappers were deleted.

Typed cleanup: in `linebreak.rs`, language/native whatsit readers now classify
to `NodePtr<WhatsitNode>`, and `typed_list_width_in` uses `mem.classify(...)`
instead of a raw node-type switch. In `main_control.rs`,
`append_penalty_node` now classifies the freshly allocated penalty node, matching
the existing `append_kern_node` pattern.

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` warnings **40**; dashboard **29/2/7/4**; `cargo build
--release -p tectonic`; full corpus
`CANDIDATE_BIN=target/release/tectonic plan/scripts/corpus.sh check all` =
**637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1`/`1905.00148v1` (script exit 1 because of expected diffs).
Metrics: `with_state` text matches **518**, `unsafe fn` **281**.

**Next:** continue draining reachable `with_state` leaves before another
`main_control_in` attempt. Good small next targets from scouting: private
shipout initializer setters, `shipout::make_xdv_glyph_array_data_in` typing to
`NodePtr<WhatsitNode>`, or more linebreak/shipout typed helper cleanup. Do not
retry a single-borrow top `main_control` until deeper wrappers are gone.

## Idiom session 229 — 2026-06-26 — Execution: collapse main-control bridge island + thread `linebreak_start`

**Type:** execution (workstream 01). Large mechanical session, intentionally
light docs.

**Code:** kept the session-228 short-borrow `main_control` shape, but removed
the extra trampoline layer: the global loop now calls existing `_in` cores
through local `mc_*` helpers (`get_x_token_in`, `scan_usv_num_in`,
`begin_token_list_in`, `show_cur_cmd_chr_in`, `main_control_*_in`,
`its_all_over_in`) and the now-dead wrappers in `main_control.rs`, `scan.rs`,
`input.rs`, and `output.rs` were deleted. Also threaded `ext::linebreak_start`
to `linebreak_start_in(&mut EngineState, ...)` and repointed
`do_locale_linebreaks_in` to it, replacing the hidden `gettexstring`/
font-state/diagnostic re-entries with direct `st.*` and output `_in` calls;
deleted the dead legacy wrapper and private global print helper.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` warnings
**42**; dashboard **29/2/7/4**; release rebuild; full corpus
`bench_phase_c.py --scope all --warmups 0 --iterations 1` over **639** cases
found only the known `1905.00073v1` / `1905.00148v1` parity failures. Metrics:
`with_state` text matches **522** with the current grep, `unsafe fn` **283**.

**Next:** continue draining reachable `with_state` leaves before attempting
`main_control_in` again. Good next targets from this session's scouting:
`io::set_input_file_encoding` / `ext::get_encoding_mode_and_info` under existing
`_in` callers, or the private shipout initializer setters.

## Idiom session 228 — 2026-06-23 — Execution: thread `make_accent` + the main-control native/ligature main loop

**Type:** execution (workstream 01). Large mechanical session, two commits.

**Commit 1 — `make_accent`.** `make_accent` → `make_accent_in(&mut
EngineState) -> Result<(), MainControlAbort>` over `scan_char_num_in`/
`new_character_in`/`do_assignments_in`/`hpack_emit_in`/`back_input_in`, the
re-entry-free native measurers (`get_native_char_{sidebearings,height_depth}(&
FontState)`), `effective_char`, and `mem::new_kern`. Folded directly into
`main_control_box_list_cases_in` (now returns `Result<(), MainControlAbort>`),
removing the `MainControlBoxListAction` trampoline. Deleted the dead
`make_accent`/`new_character`/`do_assignments`/`hpack`/`print_hpack_diag`
wrappers, the two native `_global` bridges, and `param_base`/`width_base_entry`/
`height_base_entry` globals accessors.

**Commit 2 — the native/ligature main loop.** Threaded the whole
`main_control_main_loop` subsystem into `_in` cores under one borrow each:
`main_control_main_loop_in` (AAT/OTGR native-text collection + the classic TFM
lig/kern loop), `lig_inner_loop`, `do_lookahead`,
`main_control_hmode_inter_char_check_in`, plus prerequisites
`linebreak::do_locale_linebreaks_in` and `ext::append_src_special_in`. Bulk
accessor rewrite via scripted pure-token substitution (`st.mem`/`st.eqtb`/
`st.fonts`/`st.scanner` methods; native-text now indexes
`st.scanner.native_text`), then hand-fixed the borrow-split sites
(`effective_char(&mut FontState)`), the `mc_*` alloc helpers (cold
`ALLOC_FAILED` → `MainControlAbort`), the scanner `_in` calls, the
`with_state_mut` islands (resize/inter-char/`apply_mapping`), and the
`char_warning_in`/`fix_language`/`measure_native_node`/`map_char_to_glyph`
threading. Added the matching `MemState`/`ScannerState`/`HyphState`/`FontState`/
`ShipoutState` accessor methods. Deleted ~65 now-dead bridge wrappers across
globals/scan/mem/ext/output/input/linebreak.

**KEY FINDING — `main_control` itself is NOT single-borrow threadable yet.**
First built `main_control_in(&mut EngineState)` holding ONE borrow across the
whole `big_switch`/`reswitch` loop; the dashboard caught an immediate
`with_state` nest-panic (`state.rs:1461`, align tests) because
`with_state`-bearing leaves still exist below `main_control` (Phase C is not
complete). Reverted to the **global** `main_control` loop that calls each step
(`get_x_token`/`show_cur_cmd_chr`/`main_control_main_loop`/the dispatchers/
`its_all_over`) under its **own short-lived** `with_state_mut` wrapper — exactly
the design the per-dispatch wrappers were built for. **The single continuous
borrow over `main_control` can only land at the very end of Phase C, once
nothing it reaches uses `with_state`.** Restored the thin dispatcher/scanner/
output/input wrappers that the global loop needs. Lesson also: `hi_mem_min` is
`with_state` and can shrink on get_avail's hi-mem path, so `is_char_node` must
read it fresh under the borrow (`mc_is_char_node(&EngineState)`), not capture it.

**Validation:** `cargo fmt`; `cargo check` warnings **42**; dashboard
**29/2/7/4**; release rebuild; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count ==
baseline, no NOXDV/NOLOG (the session-187 native-font check — this slice is the
native/TFM hot path). Metrics: `with_state(_mut)` **525 → 467**; `unsafe fn`
**325 → 298**.

**Next:** the main loop is threaded; the remaining `main_control` cluster is the
*global loop body itself*, which is gated on draining the last `with_state`
leaves it reaches (then `main_control_in` becomes a single-borrow pass calling
the existing `_in` cores). Scout those leaves (the dispatcher `_in` cores are
already borrow-safe; the gap is any deeper helper still on `with_state`). Until
then, keep threading individual leaves to shrink the 467 `with_state` sites.

## Idiom session 223 — 2026-06-22 — Execution: finish the main-control math dispatcher

**Type:** execution (workstream 01). Large mechanical session, intentionally
light docs.

**Code:** added `math::init_math_in` and `math::after_math_in`, then rewired
`main_control_math_cases_in` to call them directly and return
`Result<(), MainControlAbort>` (deleted the `MathCaseAction` trampoline).
Threaded the display/inline math entry and exit paths over direct
`st.mem`/`st.eqtb`/`st.input`/`st.scanner`/`st.shipout`/`st.config` access,
using the existing threaded scanner, save, packaging, pagebuilder, and math-list
builders. Added `ext::tt_insert_special_in` for semantic-pagination specials.
Deleted the now-dead bridge wrappers/accessors exposed by this slice
(`init_math`, `after_math`, `mlist_to_hlist`, `fin_mlist_global`,
`app_display`, `check_math_font_params`, `resume_after_display`,
`tt_insert_special`, `insert_src_special`, `back_error`, `line_break`,
`pop_nest`, `new_null_box_global`, `new_math_global`, `just_*_global`,
`eq_word_define_global`, and related globals).

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` warnings
**41**; dashboard **29/2/7/4** (`trip`, `tex-outputs`, `engine_rust`,
`latex_goldens`); release rebuild; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`. `trip` initially
caught a nested `nest_ptr()` re-entry in `init_math_in`; fixed by using
`st.input.nest_ptr` directly. Metrics: `with_state` matches **666**; `unsafe
fn` matches **383**.

**Next:** the math dispatcher is fully threaded. Good mechanical follow-up:
`io::open_or_close_in`; defer `main_control` itself until the remaining
dispatchers/singletons are drained.

---

## Idiom session 221 — 2026-06-21 — Execution: thread the alignment SCC, `handle_right_brace`, and the alignment dispatcher

**Type:** execution (workstream 01). Large mechanical session, minimal docs, three commits.

**Code:**
- **Commit 1 — the alignment SCC.** Threaded the mutually-recursive cluster
  `fin_align`/`align_peek`/`init_align`/`fin_row` into `_in(&mut EngineState) ->
  Result<(), MainControlAbort>` cores under one borrow each. The substrate
  (init_row/init_col/init_span/get_preamble_token/fin_col/push_alignment/
  pop_alignment/resume_after_display/do_assignments) was already threaded
  (sessions 214–220). Mechanical accessor sweep (`mem_b32s1`→`st.mem.b32s1`,
  `eqtb_b32s1`→`st.eqtb.b32s1`, nest/save/scanner globals→direct `st.input`/
  `st.save_stack`/`st.scanner`); `hpack`/`vpackage`→`hpack_emit_in`/
  `vpackage_emit_in`; `flush_math`/`flush_node_list` corrupt-node →
  `MainControlAbort::Confusion`; recoverable diagnostics print in order then
  defer via `mc_propagate_error`; arena exhaustion rides
  `NodeRecycleAbort::Overflow`. Deleted dead legacy wrappers
  (fin_align/get_preamble_token/init_row/init_col/push_alignment/pop_alignment)
  + globals accessors (nest_mode/nest_aux_b32s1/cur_align/cur_loop/cur_head/
  cur_tail_a/cur_pre_head/cur_pre_tail) + `scan::get_x_or_protected`.
- **Commit 2 — `handle_right_brace` + `fin_mlist`.** Added `math::fin_mlist_in`
  (fin_mlist + pop_nest_in under one borrow, corrupt-node → Confusion). Threaded
  `handle_right_brace` → `handle_right_brace_in`: all 15 match arms over direct
  `st.*` and the threaded callees (unsave_in/package_in/end_graf_in/
  vpackage_emit_in/align_peek_in/build_discretionary_in/build_choices_in/
  fin_mlist_in; get_token_in/end_token_list_in/back_input_in/ins_error_in;
  error/box_error/extra_right_brace via mc_propagate_error). The NO_ALIGN_GROUP/
  MATH_GROUP arms were the SCC- and fin_mlist-gated blockers, now cleared.
  Deleted the orphaned wrappers align_peek/package/build_discretionary
  (main_control), extra_right_brace/box_error (errors), build_choices_global
  (math), get_node_global (mem), vpackage/print_vpack_diag (packaging),
  page_tail/set_nest_tail (globals).
- **Commit 3 — `do_endv` + the alignment dispatcher.** Threaded `do_endv` →
  `do_endv_in` (input-stack walk via `st.input.*`, interwoven-preamble fatal via
  `InputAbort::Fatal`), the error stubs `no_align_error`/`omit_error` → `_in`,
  and added `scan::eTeX_enabled_in`. With these the whole
  `main_control_alignment_cases` dispatcher slice threads into
  `main_control_alignment_cases_in` (every arm under one borrow over
  init_align_in/eTeX_enabled_in/new_math/privileged_in/off_save_in/do_endv_in/
  cs_error_in). Deleted dead cs_error (errors), input_stack/set_input_stack/
  base_ptr/set_base_ptr (globals), no_align_error/omit_error/fin_col/fin_row/
  init_align/do_endv (main_control), eTeX_enabled (scan).

**Validation:** `cargo fmt`; `cargo check` (warnings **41**, held) per commit;
dashboard **29/2/7/4** per commit; release rebuild; full corpus **637 passed /
2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`,
doc-count == baseline, no NOXDV/NOLOG. Metrics (consistent git-grep count):
`with_state(_mut)` **760 → 732**; `unsafe fn` **→ 426**.

**Next:** the alignment subsystem is fully threaded. Remaining dispatcher
blocker is the **math cluster** (`init_math`/`after_math`/`start_eq_no`/
`scan_math`/`set_math_char`/`math_limit_switch`/`math_radical`/`math_ac`/
`append_choices`/`sub_sup`/`math_fraction`/`math_left_right`) — a coupled
multi-commit slice that gates `main_control_math_cases`. Other standalone
dispatcher verbs: `make_accent`, `insert_dollar_sign`, `align_error`, and the
mode-independent singletons `issue_message`/`show_whatever`/`do_extension`/
`open_or_close_in`. Then the remaining `main_control_*` dispatcher slices and
finally `main_control` itself.

---

## Idiom session 220 — 2026-06-21 — Execution: thread `prefixed_command` + `do_assignments`

**Type:** execution (workstream 01). Large mechanical dispatcher slice with
minimal docs.

**Code:** added `do_assignments_in(&mut EngineState)` and
`prefixed_command_in(&mut EngineState) -> Result<(), MainControlAbort>`.
The dispatcher now uses threaded scanner/output/input/hyphenation/font helpers,
direct `st.mem`/`st.eqtb`/`st.fonts`/`st.scanner` access, and local save
helpers for `eq_define`/`geq_define`/SA assignment paths. Removed the now-dead
bridge wrappers/accessors exposed by the dispatcher threading (`new_font`,
`do_register_command`, assignment verb wrappers, hyphenation/scanner/save
bridges, `after_token`, `set_box_allowed`).

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` (warnings **41**); dashboard **29/2/7/4**; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1` (script exits 1 for those known diffs).
Metrics: `unsafe fn` **448**; `with_state(_mut)` call sites **690**.

**Next:** the dispatcher blocker is gone. Thread the alignment SCC in one
batch: `fin_align_in` + `align_peek_in` + `init_align_in` + `fin_row_in`;
then `handle_right_brace_in` and the main-control dispatchers.

---

## Idiom session 219 — 2026-06-21 — Execution: thread `new_font`

**Type:** execution (workstream 01). One focused commit after the font-loader
hub.

**Code:** commit `6dcc9a6c` adds
`main_control::new_font_in(&mut EngineState, a) -> Result<(), MainControlAbort>`
and makes legacy `new_font` a thin wrapper. The threaded core uses
`open_log_file_in`, scanner `_in` calls, direct `st.strings`/`st.scanner`/
`st.fonts`/`st.eqtb` access, `string_pool::{make_string,append_str,str_eq_str}`,
`save::{eq_define,geq_define}`, and `read_font_info_in`. Added
`MainControlAbort::{OpenLog,MakeTexString,ReadFontInfo}` and deleted the now-dead
`fonts::read_font_info` bridge.

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` (warnings **41**); dashboard **29/2/7/4**; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`.

**Next:** `prefixed_command_in` + `do_assignments_in`. Subagent recon suggests a
full mechanical `prefixed_command_in` copy, first switching the low-risk branches
(`DEF_FONT`, register ops, `SET_AUX`/`SET_PREV_GRAF`/`SET_PAGE_*`/
`SET_BOX_DIMEN`, `SET_INTERACTION`) before the allocation/token-heavy branches.

---

## Idiom session 218 — 2026-06-21 — Execution: thread the font-loader hub through `read_font_info`

**Type:** execution (workstream 01). Larger mechanical font-loader batch.

**Code:** threaded the native-font lower path and the font-loader hub:
`ext::load_mapping_file_in` / `read_common_features_in` / `load_ot_font(...)`
/ `find_native_font_in(&mut EngineState, ...)`; `ext::load_tfm_font_mapping_in`;
`fonts::load_native_font_in(&mut EngineState, ...)`; and
`fonts::read_font_info_in(&mut EngineState, ...)`. Removed the newly-dead
bridge wrappers/accessors (`find_native_font`, `new_native_character`,
`load_native_font`, TFM mapping wrappers, native scratch globals, `mem_ptr`/
`set_mem_ptr`, `push_font_info_b32s1`, `quoted_filename`/
`file_name_quote_char`, etc.). `read_font_info_tfm` stays the existing threaded
TFM parser; `read_font_info_in` now calls it and the native loader under the
same borrow.

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` (warnings **40**); dashboard **29/2/7/4**; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Metrics: `unsafe fn` **471**;
`with_state(_mut)` call sites **715**.

**Next:** thread `new_font_in` in `main_control.rs`. It is now unblocked by the
font loader, but it needs a small `MainControlAbort` expansion for
`OpenLogAbort`, `MakeTexStringAbort`, `ReadFontInfoAbort`, plus direct string
pool save/restore and `eq_define`/`geq_define` calls. After `new_font_in`,
`prefixed_command_in` is again the prize.

---

## Idiom session 217 — 2026-06-21 — Execution: font-loader recon + first mapping helper

**Type:** execution (workstream 01). Small code slice after a larger attempted
font-loader thread found a lower blocker.

**Code:** split `ext::check_for_tfm_font_mapping` into a `check_for_tfm_font_mapping_in`
core plus the legacy wrapper. This is the safe bottom-most TFM mapping helper:
it only mutates the passed `name_of_file` buffer and `SAVED_MAPPING_NAME`.

**Finding:** do **not** thread `load_native_font` before `find_native_font` /
feature parsing. `find_native_font` still uses `loaded_font_*` globals and
warning paths, so calling it under a live `with_state_mut` borrow would
nest-panic. Next useful slice is lower in `ext.rs`: thread `load_mapping_file`
warning/tracing and native-font feature parsing state, then return to
`load_native_font_in` / `read_font_info_in`.

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` (warnings **41**, held); dashboard **29/2/7/4**; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`.

---

## Idiom session 216 — 2026-06-21 — Execution: thread the prefixed_command prerequisite helpers (blocked on the font loader)

**Type:** execution (workstream 01). Large mechanical session toward
`prefixed_command_in` (the session-215 "Next"). Five commits (4 threading + 1
rustfmt). Threaded every `prefixed_command` callee that lacked an `_in` core
**except** `new_font`, which a recon found is blocked on the un-threaded
font-loading subsystem.

**Code:**
- **Commit 1 — small scanner leaves** (scan.rs): `scan_char_class`/
  `scan_math_class_int`/`scan_four_bit_int`/`scan_fifteen_bit_int` → `_in` via
  `leaf_int_range_error`; `scan_xetex_math_char_int` + `get_r_token` inline
  (`get_token_in`/`back_input_in`/`ins_error_in` under the borrow). All keep thin
  `with_state_mut` wrappers (`Result<(), SccAbort>`).
- **Commit 2 — `read_toks` → `read_toks_in`** (scan.rs): one borrow over the
  `\read` scanner (`st.scanner`/`st.input`/`st.mem` direct, `get_avail_scc`/
  `get_token_in`/`begin_file_reading_in`/`input_line_in`/`end_file_reading_in`/
  `runaway_in`); `_tt_abort` + arena exhaustion ride `SccAbort::Input`. Deleted
  dead `begin_file_reading` + `buffer_entry` wrappers.
- **Commit 3 — `new_patterns_in` + `new_hyph_exceptions_in`** (hyphenation.rs):
  single-borrow `\patterns`/`\hyphenation` over `st.hyph`/`st.strings`/`st.eqtb`
  direct + `scan_left_brace_in`/`get_x_token_in`/`scan_toks_in`/`scan_char_num_in`/
  `get_avail_scc`/`make_string`/`flush_list`. Recoverable errors via
  `mc_propagate_error` (now `pub(crate)`); pool/dictionary overflow rides
  `InputAbort::Overflow`; trie overflow via a **new `MainControlAbort::Trie(TrieAbort)`**
  arm. Deleted dead hyph/strings globals accessors (`hc_entry`/`set_hc_entry`/
  `set_hyf_entry`/`trie_tr[lroc]_entry`/`hyph_index`/`set_hyph_index`/`cur_lang`/
  `set_cur_lang`/`str_pool_entry`) + the non-`_in` `max_hyphenatable_length`.
- **Commit 4 — `do_register_command_in`** (main_control.rs): single-borrow
  `\advance`/`\multiply`/`\divide` over `st.mem`/`st.eqtb`/`st.errors` direct +
  the scanner `_in` cores, `find_sa_element`/`new_spec`/`delete_glue_ref`/
  `mult_and_add_in`/`x_over_n_in`, and the save defines (`eq[_word]_define`/
  `geq[_word]_define`/`sa[_w]_def`/`gsa[_w]_def` + `finish_sa_mutation_in`).
  Arena exhaustion via `NodeRecycleAbort::Overflow`. Deleted dead
  `sa_w_def_global`/`gsa_w_def_global`; gated the test-only `mult_and_add`/
  `x_over_n` wrappers on `cfg(test)`.
- **Commit 5 — rustfmt** the slices.

**Key blocker finding (`prefixed_command` is gated on the font loader, not the
alignment SCC):** an exhaustive audit of `prefixed_command`'s callees shows the
**only** function-level blocker left is `new_font` (the `DEF_FONT` arm); every
other callee now has an `_in` core or is a trivial `with_state` field accessor
(`eqtb_b32s1`/`mem_b32s*`/`def_ref`/`avail`/`font_area_entry`/`set_help_*`/…,
rewritten inline when `prefixed_command` threads). `new_font` calls
`read_font_info` (fonts.rs, 170 lines) → `read_font_info_tfm` (381) /
`load_native_font` (149) / `load_tfm_font_mapping`/`check_for_tfm_font_mapping`
(ext.rs) — **the entire font-loading subsystem is un-threaded** and
`with_state`-heavy, so `new_font_in` can't hold one borrow across it. Recon
shows the font loaders use **localized `with_state` islands** (fonts.rs:413,
1108), **not** an OT-math-style re-entrant FFI wall (no `tt_get_font_*`
callbacks), so they thread cleanly bottom-up — but it is a multi-commit
subsystem in its own right. **`do_assignments_in` and the alignment cycle are
therefore downstream of the font loader**, correcting the session-215 "Next".

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4** per commit; release
rebuild (warnings **41**, held); full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count 639 ==
baseline, no NOXDV/NOLOG. Metrics: `unsafe fn` **475 → 470**; `with_state` sites
**740 → 730**.

**Next:** thread the **font-loading subsystem** bottom-up — `read_font_info_tfm`
/ `load_native_font` (+ `load_tfm_font_mapping`/`check_for_tfm_font_mapping` in
ext.rs) → `read_font_info` → `new_font_in`. Then `prefixed_command_in` is a
mechanical single-borrow pass (all other callees ready) → `do_assignments_in`
(trivial) → the `fin_align`/`align_peek`/`init_align`/`fin_row` cycle →
`handle_right_brace_in` → the dispatcher.

---

## Idiom session 215 — 2026-06-21 — Execution: thread fin_col + resume_after_display

**Type:** execution (workstream 01). Continued the alignment SCC. Two commits.

**Code:**
- **`fin_col` → `fin_col_in(&mut EngineState) -> Result<bool, MainControlAbort>`**
  (main_control.rs): the column-finisher under one borrow — mem/scanner/save
  direct (`cur_align`/`cur_loop`/`cur_span`/`align_state`/`total_stretch`/
  `total_shrink`/`adjust_tail`/`cur_tail` are `st.scanner.*`), packing via
  `hpack_emit_in`/`vpackage_emit_in`, `unsave_in`, `new_save_level`,
  `init_span_in`/`init_col_in`, `get_x_or_protected_in`, `pop_nest_in`. Cold
  tails ride `MainControlAbort`: `endv`/`too many spans` (`Confusion`),
  interwoven-alignment fatal (`Input(InputAbort::Fatal)`), arena exhaustion
  (`NodeRecycle::Overflow`), and the recoverable Extra-alignment-tab error
  printed in order then deferred via `mc_propagate_error`. Deleted dead
  `init_span` global wrapper + dead `cur_span`/`total_stretch` getters.
- **`resume_after_display` → `resume_after_display_in`** (math.rs, only caller
  is `fin_align`): one borrow over `unsave_in`/`push_nest_in`/`get_x_token_in`/
  `back_input_in`/`build_page_in`; `cur_group`=`st.save_stack`, `cur_lang`=
  `st.hyph`, `nest_ptr`=`st.input`, eqtb direct, cur_list scalars raw;
  `MATH_SHIFT_GROUP` guard → `Confusion`. Removed unused `cur_lang`/
  `set_cur_lang` imports.

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4**; release rebuild
(warnings **41**, held); full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count 639 ==
baseline, no NOXDV/NOLOG.

**Key structural finding (the cycle is gated on `prefixed_command`):** `fin_col`
is the ONLY alignment verb outside the `fin_align` mutual-recursion cycle
(`align_peek`→`fin_align`; `init_align`/`fin_row`→`align_peek`). So `align_peek_in`/
`init_align_in`/`fin_row_in` cannot single-borrow thread until `fin_align_in`
exists, and `fin_align` calls `do_assignments` (→ the ~1000-line
`prefixed_command` dispatcher) and `resume_after_display` (now threaded). The
agent claim that `align_peek` threads "first, trivially" is wrong — it needs
`fin_align_in`. **Next:** thread `prefixed_command` → `do_assignments_in`, then
the cycle `fin_align_in`+`align_peek_in`+`init_align_in`+`fin_row_in` lands as
one mutually-recursive commit; then `align_peek_in` → `handle_right_brace_in` →
the dispatcher.

**Raw-state note (answers "migrate remaining `raw::` state"):** the `mod raw`
block (globals.rs:52) now holds ONLY the deliberately-deferred hot-path scalars
(`cur_input`/`cur_list`/`cur_chr`/`cur_cmd`/`cur_cs`/`cur_tok`/`cur_val`/
`cur_ptr`/`temp_ptr`/`line`/`first`, the `main_*`/ligature scalars, the
`cancel_boundary`/`ins_disc`/`space_class`/`prev_class`/`bchar`/`false_bchar`
flags) + a few capacity constants (`buf_size`/`nest_size`/`save_size`/
`param_size`/`stack_size`/`max_in_open`/`expand_depth`). These are accessed via
free `cur_cmd()`-style fns that read the raw static WITHOUT a borrow precisely
so threaded `_in` cores holding `with_state_mut` can touch them without nest-
panicking. They are NOT migration debt — moving them into `EngineState` would
break the threading bridge; they retire only when ALL callers are threaded
(project end). Everything else is already in `EngineState`.

---

## Idiom session 214 — 2026-06-21 — Execution: thread main-control alignment/enabler substrate

**Type:** execution (workstream 01). Large mechanical continuation of session
213, using subagents only for recon. Targeted the bottom of the alignment SCC
and the small blockers called out in the previous handoff.

**Code:** added `_in(&mut EngineState)` cores for `scan::get_x_or_protected_in`,
`main_control::get_preamble_token_in`, `init_span_in`, `init_row_in`,
`init_col_in`, `push_alignment_in`, `pop_alignment_in`, `off_save_in`,
`head_for_vmode_in`, and `its_all_over_in`; added `errors::privileged_in`.
Deleted the dead legacy `expand::expand` wrapper and dead globals getters
`align_ptr`, `dead_cycles`, and `par_token`. The `head_for_vmode_in` par-token
case must read `st.input.par_token` directly under the borrow; the focused eTRIP
run caught the first re-entry attempt.

**Validation:** `cargo fmt -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex` (warnings **41**, held); dashboard **29/2/7/4**; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Metrics: `unsafe fn` **475**; `with_state`
sites **740** (temporary wrappers remain until callers are threaded).

**Next:** finish the alignment SCC: `fin_col_in`/`fin_row_in`/`init_align_in`/
`fin_align_in`/`align_peek_in`. Remaining blockers are `do_assignments_in`
(`prefixed_command_in`) and `resume_after_display_in`; `fin_col_in` also needs
careful packaging over `hpack_emit_in`/`vpackage_emit_in` or an extracted
non-emitting packer core.

---

## Idiom session 213 — 2026-06-21 — Execution: thread the standalone main-control dispatcher verbs

**Type:** execution (workstream 01). Large, deliberately mechanical session per
user request (bigger than prior, minimal docs). The `main_control_box_list_cases`
keystone (`handle_right_brace`/the dispatcher) is blocked on the un-threaded
alignment subsystem, so this session drained the **standalone** dispatcher verbs
that the dispatcher calls — each threaded to an `_in(&mut EngineState) ->
Result<(), MainControlAbort>` core with a thin `with_state_mut` wrapper, no
dispatcher change required. **Four commits, 12 verbs + 2 scanner cores.**

**Code:**
- **Scanner prerequisites** (scan.rs): `scan_optional_equals_in` + `scan_eight_bit_int_in`
  (the latter reuses the `leaf_int_range_error` helper). Added a `mc_propagate_error`
  helper (main_control) that maps a recoverable `ErrorOutcome` to `Ok(())` on
  `Continue` / `MainControlAbort::Error` otherwise — `?`-friendly for both the
  "error-then-return" and "error-then-continue" original control flows.
- **Commit 1** — `alter_aux`/`alter_prev_graf`/`alter_page_so_far`/`alter_integer`/
  `alter_box_dimen`, `make_mark`, `new_interaction`. `nest`/`page_so_far`/
  `dead_cycles`/`insert_penalties`/`selector`/`interaction`/`def_ref` accessed
  directly via `st.input`/`st.shipout`/`st.errors`/`st.scanner`; `cur_list`
  save/restore via raw `cur_list()`/`set_cur_list_record`. Deleted dead
  `save_cur_list_to_nest`/`set_nest_prev_graf`.
- **Commit 2** — `unpackage`, `delete_last` over threaded `find_sa_element`/
  `copy_node_list`/`free_node`/`delete_sa_ref`/`flush_node_list`/`you_cant_in`;
  deleted the dead `you_cant` global wrapper.
- **Commit 3** — `begin_insert_or_adjust`, `append_discretionary` over
  `scan_keyword_in`/`new_save_level`/`normal_paragraph`/`push_nest_in`/`new_disc`/
  `new_character_in`. Added `MainControlAbort::NewCharacter` (+`From`) for
  `new_character_in` aborts; deleted the dead `scan_eight_bit_int` global wrapper.
- **Commit 4** — `shift_case`, `append_italic_correction`. The latter calls the
  re-entry-free native measurer `real_get_native_*_italic_correction(&FontState,
  ptr)` directly under the borrow (node addr from `st.mem.data`); deleted the dead
  `real_get_native_*_italic_correction_global` wrappers + `italic_base_entry` getter.

**Still blocked (un-threaded callees, not this session):** `make_accent`
(`do_assignments`→`prefixed_command`), `head_for_vmode`/`do_endv` (`off_save` +
alignment), `its_all_over` (`privileged`), `handle_right_brace` + the dispatcher
(alignment subsystem).

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4** per commit (TRIP gates
the recoverable-error/help-line threading); release rebuild (warnings **41**, ==
baseline); full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1`/`1905.00148v1`, no NOXDV/NOLOG. Metrics: `unsafe fn` **479 → 475**;
`with_state` sites **725 → 733** (+8 transient bridge-down wrappers for the 12 new
`_in` cores — retire when the dispatcher threads and calls them directly).

**Next:** the standalone-verb vein is now drained; the remaining dispatcher verbs
and `handle_right_brace_in` are all gated on the **alignment subsystem**
(`align_peek`/`fin_align`/`init_row`/`init_col`/`fin_col`/`fin_row`/`init_align`/
`get_preamble_token` — a tight mutually-recursive SCC also needing `do_assignments`/
`resume_after_display`/`append_to_vlist`/`get_x_or_protected` threaded) or on
`prefixed_command` (the assignment dispatcher). Thread one of those keystones next,
or the smaller `off_save`/`privileged`/`do_assignments` enablers to unblock
`head_for_vmode`/`its_all_over`/`make_accent`.

---

## Idiom session 212 — 2026-06-21 — Execution: thread more box/list-DAG verbs (`unsave`/`insert_src_special`/`package`/`new_graf`/`build_discretionary`/`build_choices`/`push_math`)

**Type:** execution (workstream 01). Large mechanical continuation of the
session-211 box/list DAG. Five commits.

**Code:**
- **`unsave` → `unsave_in(&mut EngineState) -> Result<(), UnsaveAbort>`** (save.rs):
  one borrow over the level pop, inlining a threaded `sa_restore_in` loop and
  calling the threaded `eq_destroy`/`group_warning_in`/`back_input_in` under it;
  `cur_tok`/`cur_input` stay raw `mod raw`. New `UnsaveAbort` (Save/Input/
  MainMemory) fired by the thin wrapper. Added `SaveStackState` `b16s1`/`b16s0`/
  `word` readers. Deleted dead `sa_restore_global`/`eq_destroy_global`/
  `group_warning` + dead globals accessors (`save_stack_b16s0`/`b16s1`/`word`,
  `set_save_stack_word`, `cur_boundary`, `grp_stack`, `xeq_level`).
- **texmfmp src-special helpers** `is_new_source`/`make_src_special`/
  `remember_source_info` → safe `_in` cores (narrow unsafe around the FFI), then
  **`insert_src_special` → `insert_src_special_in`** over `str_toks_cat_in`/
  `get_avail`/`begin_token_list_in` with the `InsertSrcAbort` sentinel.
- **`package` → `package_in`** and **`new_graf` → `new_graf_in`**: each one borrow
  over `unsave_in`/`hpack_emit_in`/`vpackage_emit_in`/`pop_nest_in`/`box_end_in`
  and `push_nest_in`/`new_param_glue`/`new_null_box`/`insert_src_special_in`/
  `build_page_in`. Extended `MainControlAbort` with `Unsave`/`InsertSrc` arms.
  Deleted dead `box_end`/`cur_box`/`set_cur_box`/`insert_src_special_every_par`.
- **`push_math` → `push_math_in`**, **`build_choices` → `build_choices_in`**
  (math.rs), **`build_discretionary` → `build_discretionary_in`**: each one borrow
  over the threaded `unsave_in`/`fin_mlist`/`pop_nest_in`/`new_save_level`/
  `scan_left_brace_in`/`push_nest_in` and the output/error `_in` diagnostics;
  recoverable `error()` sites propagate via `MainControlAbort::Error`.

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4** per commit; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1`/`1905.00148v1`, no NOXDV/NOLOG (new_graf/package are hot-path).
Metrics: warnings **41** (held); `unsafe fn` **480 → 479**; `with_state` sites
**734 → ~725**.

**Next (handle_right_brace_in + the dispatcher are blocked on the alignment
subsystem):** `handle_right_brace_in`'s `NO_ALIGN_GROUP`/`ALIGN_GROUP` arms call
`align_peek`, which pulls in the **un-threaded alignment subsystem**
(`fin_align`/`init_row`/`init_col`/`get_x_or_protected`/`normal_paragraph`) — a
separate multi-session slice. Likewise `main_control_box_list_cases` dispatches
~20 verbs, many alignment-dependent (`head_for_vmode`/`begin_insert_or_adjust`/
`make_mark`/`delete_last`/`unpackage`/`append_discretionary`/`make_accent`/
`align_error`/`off_save`/`indent_in_hmode`/`normal_paragraph`/`scan_dimen`).
**Thread the alignment subsystem next** (then `align_peek_in` → `handle_right_brace_in`),
or pick off the remaining standalone dispatcher verbs (`make_mark`/`delete_last`/
`unpackage`/`append_discretionary`/`make_accent`) one at a time. The
`build_discretionary_in`/`build_choices_in` cores already exist for the future
`handle_right_brace_in` DISC/MATH_CHOICE arms.

---

## Idiom session 211 — 2026-06-21 — Execution: thread the box/list verb DAG (`box_end`/`begin_box`/`scan_box`/`end_graf`)

**Type:** execution (workstream 01). Large mechanical session: the
`main_control_box_list_cases` set is a **DAG**, not a tight SCC, so it threads
bottom-up. Threaded the substrate + the box-creation/box-disposal verbs.

**Code:**
- **Substrate cores:** `mem::push_nest_in` (+ `PushNestAbort`), `scan::scan_rule_spec_in`,
  `scan::scan_spec_in` (save-stack overflow → `SccAbort`), `packaging::vsplit_in`
  (+ `VsplitAbort`; threaded `print_vsplit_packages_in`/`print_vsplit_needs_vbox_error_in`
  on the existing `print_vpack_diag_in`/`print_vert_break_infinite_shrinkage_error_in`).
- **`line_break_in` plumbing:** made `line_break_in` + `LineBreakOutcome` (and the
  nested `TryBreakAbort`/`HyphenateAbort`/`HyphNativeAbort`/`PostLineBreakOutcome`)
  `pub(crate)`; extracted `fire_line_break_outcome` so threaded callers can defer
  the cold abort.
- **`save::finish_sa_mutation_in`** (+ `SaMutationAbort`): threaded form of the
  `sa_def`/`gsa_def` finish (overflow/flush/`delete_sa_ref`) for `box_end`.
- **`MainControlAbort`** extended with `LineBreak`/`PushNest`/`Vsplit`/`Input`/
  `Shipout`/`Save`/`SaMutation`/`Error`/`Confusion` arms (+ `From` impls + `fire`).
- **Verbs → `_in(&mut EngineState) -> Result<(), MainControlAbort>`:** `end_graf_in`,
  `box_end_in`, `begin_box_in`, `scan_box_in`. Each holds one borrow, calling the
  threaded scanner/input/pagebuilder/shipout/save/packaging `_in` cores under it;
  `cur_box`/`adjust_tail`/`pre_adjust_tail` are `st.scanner` fields, `cur_list*`/
  `cur_ptr`/`cur_val` stay raw `mod raw`. Deleted the now-dead `ship_out`,
  `vsplit`, `new_rule_global` global wrappers + the dead `max_nest_stack` getter.

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4** per wave; release
rebuild; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1`/`1905.00148v1`, **no NOXDV/NOLOG/NEW/MISS** (box verbs are on the
hot path). Metrics: warnings **42** (rust-only, == baseline); `unsafe fn`
**481 → 480**; `with_state` sites **807 → 734**.

**Next:** the rest of the DAG — `new_graf_in` (needs `insert_src_special` threaded,
which pulls in texmfmp `make_src_special`/`is_new_source`/`remember_source_info`
`_in` cores + `str_toks_cat_in`), `package_in` (needs `unsave_in`, which needs
`sa_restore`/`eq_destroy`/`group_warning_in`), then `handle_right_brace_in`
(needs `unsave`/`build_discretionary`/`align_peek`/`build_choices`/`fin_mlist`/
`box_error` threaded) and finally the `main_control_box_list_cases` dispatcher.

---

## Idiom session 210 — 2026-06-20 — Execution: thread main-control append leaves and DVI finalizer

**Type:** execution (workstream 01). Low-docs mechanical continuation on top of the already-tested session-209 pagebuilder WIP.

**Code:** Added `MainControlAbort` and threaded the append leaf verbs `append_penalty`/`append_glue`/`append_kern` into `_in(&mut EngineState)` cores using scanner `_in` callees and `build_page_in` where vertical penalties trigger page building. Made `pagebuilder::{PageBuildAbort, build_page_in, fire_page_build_abort}` crate-visible for threaded callers. Threaded `shipout::finalize_dvi_file` into a single-borrow `finalize_dvi_file_in`, using direct `ShipoutState`/`FontState` access plus `emit_dvi_*`, `dvi_font_def_in`, `write_to_dvi`, and output `_in` diagnostics. Removed the now-dead DVI finalizer bridge wrappers/accessors (`dvi_*_global`, shipout read accessors, `make_font_def_global`, `prepare_mag`, and retired shipout/font getters in `globals.rs`).

**Validation:** `cargo check -p tectonic_engine_xetex`; dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus `crates/engine_xetex/plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1` (script exit 1 due to expected divergences). Metrics: warnings **41**; `unsafe fn` **481**; `with_state` sites **807**.

**Next:** continue the `main_control_box_list_cases` page/list SCC (`new_graf`/`end_graf`/`scan_box`/`begin_box`/`box_end`/`package`/`handle_right_brace`). `build_page_in` is now callable; remaining blockers include `line_break_in` visibility/abort plumbing and `insert_src_special` threading.

---

## Idiom session 209 — 2026-06-20 — Execution: thread the pagebuilder hub

**Type:** execution (workstream 01). Large low-docs mechanical threading pass.

**Code:** Threaded `pagebuilder.rs`'s `build_page`/`fire_up` hub into `build_page_in(&mut EngineState) -> Result<(), PageBuildAbort>` and `fire_up_in(&mut EngineState, ...)`, leaving `build_page()` as a thin abort-firing wrapper. Added `PageBuildAbort` plumbing for input/save/SCC/shipout/node-recycle/vert-break/main-memory/confusion exits, threaded the local pagebuilder helpers, and replaced the hot global bridge calls with direct `st.*` access plus existing `_in` callees. Added `packaging::vert_break_in` and made `shipout::ship_out_in`/`fire_shipout_abort` callable from pagebuilder. Removed now-dead pagebuilder/globals bridge accessors and obsolete `vert_break`/`prune_page_top_global` wrappers.

**Validation:** `cargo check -p tectonic_engine_xetex`; dashboard **29/2/7/4** (`tex-outputs`, TRIP+eTRIP, `engine_rust`, `latex_goldens`); `cargo build --release -p tectonic`; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Fixed the only runtime nest-panic found during validation by using `scaled_math::x_over_n_in` inside the held pagebuilder borrow. Metrics: warnings **41**; `unsafe fn` **486**; `pagebuilder.rs` now has one real `with_state_mut` wrapper around `build_page_in` (other `with_state` hits are comments).

**Next:** continue with another large hub now unlocked by scanner/pagebuilder threading, most likely `main_control.rs`'s main loop or adjacent page/output consumers.

---

## Idiom session 208 — 2026-06-20 — Execution: thread the mlist_to_hlist SCC

**Type:** execution (workstream 01). Threaded the math SCC now that it is unblocked by the OT-math FFI mirror solution. One large commit.

**Code:**
- **`mlist_to_hlist` SCC** fully threaded: `mlist_to_hlist` converted to `mlist_to_hlist_in(st: &mut EngineState) -> Result<(), MathBuildAbort>`.
- **Math builders** threaded: `make_over`, `make_under`, `make_vcenter`, `make_radical`, `make_fraction`, `make_op`, `make_ord`, `make_math_accent`, `make_left_right`, `make_scripts`, `finish_subscript_only`, `finish_superscript_branch`, `initial_script_shifts`, `finish_subsup_stack`, `maybe_attach_script_kern`, `mlist_advance` converted into `_in` cores under a single borrow.
- **Diagnostics**: Added `vpackage_emit_in` (packaging.rs) to handle vertical box diagnostics without re-entering `with_state`.
- **Sentinels**: Added `MathBuildAbort` outcomes (including propagation of `ErrorOutcome` from math diagnostic warnings) and dissolved math `_global` bridge wrappers (e.g. `char_box_global`, `rebox_global`, `build_opentype_assembly`, `var_delimiter_global`).
- **Cleanup**: Deleted unused getters/setters (`cur_f`, `cur_c`, `cur_i`, `cur_mlist`, `cur_style`, `cur_size`, `cur_mu`, `mlist_penalties`) from `src/globals.rs`.

**Validation:** `cargo check`+`fmt`; dashboard **29/2/7/4** (TRIP+eTRIP green); release rebuild; full corpus verified by user, exit 0, `unicode_math` passes. Metrics: warnings **42 → 41**; `unsafe fn` **496**; `math.rs` `with_state` sites **168 → 22** (−146); total `with_state` sites **766** (down from 984).

**Next:** thread `pagebuilder.rs` (`build_page`/`fire_up`) or `main_control.rs` (`main_control`) hubs.

---

## Idiom session 207 — 2026-06-20 — Execution: thread the OT-math-FFI re-entry wall, then recover the reverted math substrate

**Type:** execution (workstream 01). Took the session-206 "Next": thread the
OT-math-FFI wall so the `mlist_to_hlist` SCC becomes threadable. Three commits.

**Code:**
1. **The wall (keystone).** The OpenType-math C++ FFI (`xetex-XeTeXOTMath.cpp`)
   reads `font_size`/`font_area`/`font_layout_engine` back via the three
   `tt_get_font_{size,area,layout_engine}_entry` callbacks, which took the
   `ENGINE_STATE` borrow (`with_state`) — so any threaded `_in` core holding
   `with_state_mut` across an OT-math call nest-panicked (session-206 wall). Gave
   the three callbacks a **borrow-free** read path: raw mirror pointers
   (`globals::font_ffi_mirror`, `mod raw`-style like `line()`/`cur_input`)
   refreshed by `refresh_font_ffi_mirrors(&FontState)` at the **only 3
   reallocation sites** of those Vecs (engine init `engine.rs`, format undump
   `format_io.rs`, cleanup `engine.rs`). The Vecs are sized to `font_max + 1` and
   never grow mid-run (`font_ptr` is bounded by `font_max` — verified: no
   push/resize/take/replace anywhere), so in-place `set_font_*_entry` writes keep
   the mirrors observing live values. The other ~175 read/write sites of the
   three arrays are untouched (they stay on `FontState`); only the FFI callbacks
   use the mirror.
2. **+3. Recovered the reverted substrate.** Session 206 had *written* the
   substrate threading (`char_box`/`stack_glyph_into_box`/`native_glyph_vbox`/
   `stack_glue_into_box`/`build_opentype_assembly` → `_in` cores +
   `MathBuildAbort`; then `var_delimiter`/`rebox` → `_in`) and reverted both
   **only** because they crossed the now-fixed wall (`var_delimiter_in` put
   `axis_height` inside the borrow; `build_opentype_assembly_in` held the borrow
   across `ot_part_*(f,…)`). With the wall down these are correct, so recovered
   them by `git cherry-pick -x c6febe37 3e8dca78` (both touch only `math.rs`,
   disjoint from the keystone) — clean apply, no conflicts.

**Validation (the keystone's regression is corpus-only — no OT-math font in the
dashboard, the session-187/206 doc-count rule):** dashboard **29/2/7/4** after
the keystone and again after the substrate; release rebuild; full corpus
**637 passed / 2 diverged / 0 missing-golden**, only the known
`1905.00073v1`/`1905.00148v1` FP-math divergences — doc-count 639 == baseline,
**`unicode_math` passed** (it exercises the OT-math FFI through the mirror).
Metrics: warnings **42** (held); `unsafe fn` **496** (−; the recovered substrate
deleted dead `_global` wrappers); `math.rs` `with_state` sites **183 → 168**.

**Next:** the wall is gone and the delimiter/native-glyph substrate is threaded —
the `mlist_to_hlist` **SCC** is now unblocked and is the prize. It is the
mutual-recursion knot `mlist_to_hlist ↔ clean_box ↔ make_*` (no smaller safe
slice: `clean_box` calls `mlist_to_hlist`, `mlist_to_hlist` calls the `make_*`
which call `clean_box`), so it threads as **one** big commit — ~2000+ lines:
`mlist_to_hlist` (427), `clean_box` (~75), and the `make_*` family
(`make_over`/`make_under`/`make_vcenter`/`make_radical`/`make_fraction`/`make_op`
(~350)/`make_ord`/`make_math_accent`/`make_left_right`/`make_scripts`); `fetch`
already has `fetch_in`, and the delimiter/box substrate (`var_delimiter_in`/
`rebox_in`/`char_box_in`/`build_opentype_assembly_in`/…) is now in place to call
under the borrow. Budget it as a focused multi-turn push (the session-204 scanner
SCC is the precedent); validate by the **full corpus** (`unicode_math` must stay
passed), not just the dashboard. The OT readers are now safe under a held borrow,
so the session-193/206 "compute `axis_height`/OT params outside the borrow"
workaround is no longer required.

---

## Idiom session 206 — 2026-06-20 — Recon (failed attempt + revert): the `mlist_to_hlist` SCC is blocked by an OT-math-FFI re-entry wall

**Type:** execution attempt → **reverted**, no net code change. Took the session-205
"Next" (thread the `mlist_to_hlist` SCC). The SCC is the mutual recursion
`mlist_to_hlist → make_* → clean_box → mlist_to_hlist`, so (like the scanner SCC)
it can only become single-borrow `_in` cores if all its substrate is threaded
first. Threaded the substrate in two commits — the native-glyph/OT delimiter
builders (`char_box_in`/`stack_glyph_into_box_in`/`native_glyph_vbox_in`/
`stack_glue_into_box_in`/`build_opentype_assembly_in`) and `var_delimiter_in`/
`rebox_in` — dashboard 29/2/7/4 on both.

**Why reverted (the wall):** the full corpus caught **`NOXDV unicode_math`**
(636 passed vs. baseline 637 — the session-187 doc-count check). Root cause: the
OpenType-math FFI in `xetex/xetex-XeTeXOTMath.cpp` (`get_native_mathsy_param`,
`get_native_mathex_param`, `get_ot_math_constant`, `get_ot_math_variant`,
`get_ot_assembly_ptr`, `ot_min_connector_overlap`, `ot_part_{start,end}_connector`,
`ot_part_full_advance` — every wrapper passed a **font id** `f`) calls back into
Rust via `tt_get_font_size_entry` / `tt_get_font_area_entry` /
`tt_get_font_layout_engine_entry`, and **all three callbacks are `with_state`
accessors** (`globals.rs` → `state.fonts.font_{size,area,layout_engine}[f]`). So a
threaded `_in` core that holds `with_state_mut` and calls any of these
**nest-panics** (`RefCell already borrowed`), producing no `.xdv`. This is exactly
why the pre-existing island code computes `axis_height` and the OT readers
*outside* every `with_state` island (e.g. the session-193 `make_vcenter` note);
`var_delimiter_in` moved `axis_height(s)` inside the borrow and
`build_opentype_assembly_in` held the borrow across `ot_part_*(f,…)`.

**This is a hard wall, the math analog of the session-187 native-measurer wall.**
Crucially it blocks the **whole** SCC, not just the delimiter builders: every
`make_*` arm that touches OT math (`make_radical`/`make_op`/`make_fraction`/
`make_left_right`/`make_math_accent`/`make_vcenter` via `var_delimiter` /
`get_ot_math_*` / `axis_height` / `math_*_param`) re-enters, so `mlist_to_hlist_in`
holding one borrow and dispatching to any of them panics. No partial SCC threading
is possible. ~150 of math.rs's 183 `with_state` sites are gated on this.

**What IS safe (re-confirmed):** the native-glyph path is fine —
`new_native_character_in` is `&mut EngineState` and `measure_native_glyph` is
`&FontState`, both re-entry-free (session 187). Only the **OT-math** FFI (the
`xetex-XeTeXOTMath.cpp` family above) re-enters. So `char_box_in` itself was
correct; it was bundled with the OT-crossing functions and reverted with them.

**The prerequisite (next session's real target):** make the three
`tt_get_font_{size,area,layout_engine}_entry` callbacks **re-entry-free** so the
OT-math FFI is callable under a held borrow (then the SCC threads mechanically,
exactly as session 187 unblocked `line_break`/`mlist`). The arrays are
`Vec<…>` fields on `FontState` inside the `ENGINE_STATE` `RefCell`. Design options:
(a) raw mirror pointers (`mod raw`-style, like `line()`/`cur_input`) for the three
arrays, refreshed after every font-table growth (find **all** push sites in
`fonts.rs`/`ini.rs` — a missed resync is silent UB, so this needs the full corpus
+ a deliberate font-definition-during-math test); (b) move just those three arrays
out of the `RefCell` into a separate callback-readable home. Validate with the
**full corpus doc-count** (`unicode_math` must stay passed), not just the dashboard
— the dashboard has no OpenType-math font and never exercises the wall.

**Tests:** dashboard 29/2/7/4 held throughout; the regression was corpus-only
(`NOXDV unicode_math`), caught by the doc-count check. After revert: `cargo check`
clean, `src/math.rs` byte-identical to baseline `5f526487`. History carries the two
threading commits + their two reverts (net-zero src; `git reset` unavailable).

**Next:** thread the OT-math-FFI re-entry wall (the three `tt_get_font_*`
callbacks) per the prerequisite above; **then** the `mlist_to_hlist` SCC
(`mlist_to_hlist`/`clean_box`/`make_*`/`var_delimiter`/`build_opentype_assembly`)
threads as one commit over the now-safe substrate. Until then, do **not** retry the
SCC. A safer interim target with no OT-math coupling: `pagebuilder.rs`/`output.rs`
tails (the scanner is now threaded, session 204, so `output.rs`
`show_whatever`/`issue_message` may be unblocked).

---

## Idiom session 205 — 2026-06-20 — Execution: finish the scanner-gated shipout hub

**Type:** execution (workstream 01). Continued the interrupted `src/shipout.rs`
session and finished the hub unlocked by session 204's scanner SCC.

**Code:** `ship_out`, `hlist_out`, and `vlist_out` are now thin wrappers over
`_in(&mut EngineState) -> Result<(), ShipoutAbort>` cores. The core path keeps
recursive hlist/vlist traversal, rules, leaders, native glyph/font DVI emission,
`pic_out`, `out_what`/`write_out`, SyncTeX, page setup, log/open-name handling,
and DVI movement/output under the held borrow. Fixed `tex-outputs` nest-panics
from leftover global accessors (`gettexstring`, `is_char_node_global`,
`doing_leaders()`, `dvi_offset()`/`dvi_ptr()`) by using threaded/direct state
access. Removed obsolete shipout bridge wrappers and imports made dead by the
hub conversion.

**Tests:** `cargo check -p tectonic_engine_xetex`; `tex-outputs` **29/29**;
`trip` **2/2**; `engine_rust` **7/7**; `latex_goldens` **4/4**; release rebuild;
full corpus **637 passed / 2 diverged / 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`.

**Next:** take another large now-unblocked hub, probably `mlist_to_hlist` or an
output/pagebuilder slice with remaining raw/global bridge access.

---

## Idiom session 204 — 2026-06-20 — Execution: the scanner-SCC big pass

**Type:** execution (workstream 01). Executed the **scanner / `expand` SCC big
pass** per `scanner-scc-plan.md` §"Execution manifest (session 203 recon)":
threaded the 37-function mutually-recursive cluster headed by `expand` into
`_in(&mut EngineState) -> Result<T, SccAbort>` cores, all converting together so
the SCC holds **one** borrow per entry (`01b` §4).

**Code (single large commit — the SCC has no safe smaller bisect unit, user-approved):**
- **`SccAbort` sentinel** (scan.rs, near `GetNextOutcome`): arms `Error`/`Input`/
  `NoLegalEnd`/`TokensToString`/`PdfError`/`OpenLog`/`MakeTexString`; `fire()` +
  `From<GetNextOutcome>`/`From<InputAbort>`. Helpers `scc_err`/`get_next_scc`/
  `str_toks_scc`/`get_node_scc`/`get_avail_scc`/`new_spec_scc`/`main_mem_overflow_scc`.
- **37 `_in` cores** filled (translated from the global versions): the 9 trivial
  range-check/delegator leaves; `x_token`/`get_x_token`/`get_token`(new helper)/
  `scan_left_brace`/`scan_keyword`/`scan_general_text`/`scan_glyph_number`/
  `scan_font_ident`/`find_font_dimen`/`compare_strings`/`pass_text`/`pseudo_start`/
  `the_toks`/`ins_the_toks`/`scan_and_pack_name`/`scan_file_name`/
  `scan_file_name_braced`; the large `scan_int`/`xetex_scan_dimen`/`scan_glue`/
  `scan_expr`/`scan_toks`/`conditional`/`expand`/`macro_call`/
  `scan_something_internal`(~900 lines)/`conv_toks`(~600 lines)/`start_input`.
  Each global is now a thin `with_state_mut(|st| f_in(st,…))` wrapper for its
  external (non-SCC) callers.
- **Below-SCC re-entry hazards threaded** (manifest §b nest-panic audit, caught
  by the dashboard/corpus at runtime): `tt_xetex_open_input`/`u_open_in` →
  `_in` cores (io.rs); the texmfmp date/file pool writers
  `getcreationdate`/`getfilemoddate`/`getfilesize`/`getmd5sum`/`getfiledump` →
  `_in(&mut StringsState)` cores; `count_pdf_file_pages` → `count_pdf_file_pages_named`;
  `gr_print_font_name`/`print_glyph_name` → `_in` cores (ext.rs);
  `not_font_error_common_in` made `pub(crate)`. The scaled-math side-effecting
  helpers `mult_and_add`/`add_or_sub`/`quotient`/`fract` (+ `xn_over_d`) gained
  `_in(&mut ErrorState)` cores so the SCC (and `linebreak::try_break`) can call
  them under a borrow instead of via the `set_arith_error`/`set_tex_remainder`
  `with_state` path; their globals stay thin wrappers.
- **Cleanup (manifest step 6):** deleted **76** now-dead functions the conversion
  orphaned (35 `globals.rs` accessors — `max_reg_num`/`max_reg_help_line`/
  `cur_order`/`radix`/`scanner_status`/… ; the 14 dead SCC global wrappers; dead
  `not_*_font_error`/`mu_error`/`change_if_limit`/`print_cs`/`print_spec`/
  `show_token_list`/`prev_rightmost_global`/`*_global` font getters/…).

**Recovery note (for future sessions):** the first wiring attempt used a naive
brace-counter that desynced on `{`/`}` inside C-string literals (help-lines like
`\def\a1{...}`), corrupting several globals (e.g. it ate `check_outer_validity_in`).
Recovered by reconstructing scan.rs/expand.rs/io.rs from the git baseline with a
**literal/comment-aware** brace matcher for the wiring, then grafting the intact
`_in`-core blocks back. Lesson: any scripted Rust body-replacement MUST skip
string/char/comment spans when matching braces.

**Tests:** `cargo check`+`fmt`; dashboard **29/2/7/4** (TRIP+eTRIP green); release
rebuild; full corpus exit 0, only the known `1905.00073v1`/`1905.00148v1`
divergences, **no NOXDV/NOLOG/NEW/MISS** (the session-187 file-name/`\input`
hidden-regression check — start_input/conv_toks/scan_file_name were all on the
hot path here). Metrics: warnings **41 → 39**; `unsafe fn` **529 → 506**;
`with_state(_mut)` sites **1014 → 984**.

**Next:** the scanner is threaded. `get_x_token`/`scan_toks`/`the_toks` now have
`_in` cores, unlocking the **scanner-gated shipout hub** (`ship_out`/`hlist_out`/
`vlist_out` via `out_what`→`write_out`→`scan_toks`/`get_token`) — the
session-191 DVI-emit layer is the reusable substrate. Residual cleanup: 8 cosmetic
`unnecessary unsafe` warnings in `scan_something_internal_in`'s FFI-getter arms.

---

## Idiom session 203 — 2026-06-19 — Execution: drain the scanner-SCC prerequisite tail

**Type:** execution (workstream 01). Bigger-than-usual mechanical session per user
request. Continues the **scanner-SCC** plan (`scanner-scc-plan.md`): thread every
below-SCC `with_state`-bearing helper the 35-function `expand` SCC calls, so the
SCC can later convert under a single borrow without nest-panicking (`01b` §4).

**Inherited (session 202, committed but unlogged):** the `scanner-scc-plan.md`
analysis (Tarjan SCC + hazard audit) + two prereq commits — `str_toks_cat` →
`str_toks_cat_in` (`StrToksAbort`), and the io.rs file-name machinery
(`make_utf16_name`/`begin_name`/`more_name`/`end_name`/`make_name_string`) → `_in`
cores (`NameAbort`). Verified green and folded into this session's baseline.

**Code (3 commits this session):**
1. **`round_decimals` + `tokens_to_string`** → `_in` cores. `round_decimals(&ErrorState,k)`
   reads `dig` directly (its only caller `xetex_scan_dimen` is still raw → bridge-down
   `with_state` wrap). `tokens_to_string_in(&mut EngineState,p)` renders the token list
   via threaded `show_token_list_in`; the `new_string`-reentrancy guard + string-pool
   overflow ride a new `TokensToStringAbort` (`01b` §5). Deleted dead `dig_entry`.
2. **`prepare_mag`** → `prepare_mag_in(&mut EngineState) -> ErrorOutcome`. Validates
   `\mag` under one borrow via `error_here_with_diagnostic_in`/`int_error_in`/print
   cores + threaded `geq_word_define`; both recoverable `int_error` sites propagate a
   non-`Continue` outcome to the wrapper's `finish()` (the original longjmps before the
   `geq_word_define`, so the early return matches). Deleted dead `mag_set` getter.
3. **`pack_file_name`/`pack_job_name`/`open_log_file`/`maketexstring`** → `_in` cores —
   the whole `\input`/log-open file-name path (the session-187 hidden-regression risk).
   `maketexstring_in(&mut StringsState,…)` does the UTF-8→pool write directly (new
   `MakeTexStringAbort` + `fire_make_tex_string`); `pack_file_name_in`/`pack_job_name_in`
   build `name_of_file` via `string_pool::length`/`gettexstring_in`; `open_log_file_in`
   opens + writes the banner under one borrow via `make_name_string_in` + print cores,
   with a new `OpenLogAbort` sentinel (MakeString/Name/CannotOpen). All four keep thin
   `with_state` wrappers for their many non-SCC callers (pic/fonts/shipout/engine/…).
   Deleted dead `set_log_file`/`set_texmf_log_name` accessors.

This completes the documented prereq list (`scanner-scc-plan.md` §"Still to thread"):
`prepare_mag`, `tokens_to_string`/`round_decimals`, and the `start_input`
file-name/log path are all threaded. The SCC's downward callee tail is now drained.

**SCC big-pass recon (prototyped + reverted, manifest landed).** Per user direction
("one big push"), prototyped the SCC conversion — added the `SccAbort` sentinel
(mirrors `GetNextOutcome`'s non-`Done` arms; `fire()` + `From<GetNextOutcome>`) and
converted the 9 trivial leaf/delegator members (`scan_char_num`/`scan_usv_num`/
`scan_char_class_not_ignored`/`scan_math_fam_int`/`scan_register_num`/
`scan_four_bit_int_or_18` + `scan_normal_glue`/`scan_mu_glue`/`scan_pdf_ext_toks`).
**Then reverted the source WIP:** the SCC is one all-or-nothing compile+validate unit
(~4000 lines incl. the `expand`/`macro_call`/`conditional`/`scan_something_internal`
~900-line monsters) — genuinely multi-turn, and a half-done *wired* push leaves
`todo!()` stubs in live wrappers (uncommittable, test-failing). Captured everything
learned as a concrete **execution manifest** in `scanner-scc-plan.md` so the real pass
is mechanical: the `SccAbort` code, the **nest-panic audit rule** (raw-safe `mod raw`
set vs `with_state` accessors that must become `st.<cluster>.field` — landmines found:
`max_reg_num`/`max_reg_help_line`/`set_help_*`/`selector`/`align_state`), the
leaf-scanner exemplar, the not-an-SCC-member list (`scan_eight_bit_int` & kin),
per-function return types, member-size budget, and the **incremental-compilation
strategy** (stub all 35 → wire all 35 wrappers → fill bodies with `cargo check` per
fn → validate by corpus only at the end).

**Tests:** `cargo check` + `fmt`; dashboard **29/2/7/4** (TRIP green); release rebuild;
full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/
`1905.00148v1`, doc-count 639 == baseline, **no NOXDV/NOLOG** (the file-name/`\input`
hidden-regression check). Metrics: warnings **41** (held); `unsafe fn` **529**,
`with_state(_mut)` sites **1014** — both up slightly from the new `_in`+wrapper pairs
for the FFI/pool-heavy prereqs; they retire when the SCC threads and the wrappers drop.

**Next:** the **SCC big pass** — now fully specced in `scanner-scc-plan.md`
§"Execution manifest (session 203 recon)". Follow the incremental-compilation
strategy there (stub all 35 → wire all 35 wrappers → fill bodies, `cargo check` per fn
→ validate by full corpus only at the end: 637/2/0, doc-count == baseline, **no
NOXDV/NOLOG**). The `SccAbort` design + the per-accessor nest-panic audit rule + the
leaf-scanner exemplar are all in the manifest. Budget it as a multi-turn push (the 9
trivial members are mechanical; `scan_something_internal`/`expand`/`macro_call`/
`conditional`/`scan_dimen`/`scan_expr` are the real work). Threading the scanner unlocks
the scanner-gated shipout hub (`ship_out`/`hlist_out`/`vlist_out` via `out_what`→`write_out`).

---

## Idiom session 201 — 2026-06-18 — Execution: thread `get_next` (the central tokenizer)

**Type:** execution (workstream 01). Took the session-200 "Next": thread the
~630-line `get_next` hub into a single-borrow `get_next_in(&mut EngineState)`,
after first threading its last blocker `input_line` (io.rs). Two commits.

**Code (2 commits):**
1. **`input_line` + io helpers** → `&mut EngineState` `_in` cores
   (`input_line_in`/`get_uni_c_in`/`apply_normalization_in`/
   `conversion_error_in`). Buffer reads/writes are direct `st.input.*`; input
   normalization reads `st.eqtb` directly; the bad-UTF8 / conversion-error
   diagnostics go through the threaded `bad_utf8_warning_in` / print cores. The
   cold `_tt_abort` paths (buffer-too-small / invalid file mode / synthetic
   terminal) ride a new `InputAbort::TtAbort` sentinel fired after the borrow
   drops. `input_line()` stays a thin wrapper for its three still-global callers.
   Deleted dead `buffer_ptr`/`eqtb_is_null`/`bad_utf8_warning` wrapper +
   `scan::get_input_normalization_state`.
2. **`get_next`** → `get_next_in(st) -> GetNextOutcome`; `get_next()` is now a
   thin `with_state_mut` wrapper. The core holds **one** borrow across the whole
   read: buffer/eqtb/mem/scanner/input/save state touched directly; the threaded
   input-stack/error `_in` cores (`begin_token_list_in`, `end_token_list_in`,
   `end_file_reading_in`, `check_outer_validity_in`, `pseudo_input_in`,
   `input_line_in`, `file_warning_in`, `error_in`/`error_here_with_diagnostic_in`/
   `capture_to_diagnostic_in`, `print_char_in`, `id_lookup`) run under it;
   `cur_input`/`cur_chr`/`cur_cmd`/`cur_cs`/`line`/`first` stay raw `mod raw`
   (safe). Cold tails ride a new **`GetNextOutcome`** sentinel fired by the
   wrapper after the borrow drops: `Error(ErrorOutcome)` (recoverable-error /
   outer-validity `finish()`), `Abort(InputAbort)` (input-stack cold abort, incl.
   `id_lookup` overflow + interwoven-alignment `fatal_error`), and
   `NoLegalEnd { open_log }` (the `\end`-less `open_log_file` + `fatal_error`
   path). The bulk accessor rewrite was driven by a one-shot balanced-paren
   transform (deleted after use), then the `_in`-call / control-flow conversions
   were finished by hand. Deleted now-dead globals `param_stack`/`eof_seen`/
   `set_eof_seen`/`force_eof`/`used_tectonic_coda_tokens` accessors and the
   `pseudo_input`/`file_warning`/`check_outer_validity` global wrappers.

**Tests:** `cargo check` + `fmt`; dashboard **29/2/7/4** after each commit (TRIP
exercises `get_next` heavily — the regression gate); release rebuild; full corpus
**637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/
`1905.00148v1`, no NOXDV/NOLOG, doc-count == baseline. Metrics: warnings **41**
(held); `unsafe fn` **527 → 520** (deleted wrappers > added input_line_in/
get_uni_c_in); `with_state(_mut)` sites **~1093 → 1012** (get_next no longer
calls the bridge accessors + deleted dead accessors).

**Next:** `get_x_token` is gated on threading `expand` (it re-enters
`with_state`); `get_token_in` would be speculative until a threaded caller exists.
The natural next hub is **`expand`** (then `get_x_token`/`scan_toks`/`the_toks`,
which unlock the scanner-gated shipout hub `ship_out`/`hlist_out`/`vlist_out` via
`out_what`→`write_out`). `get_next_in` is now a clean pass-through for any future
threaded scanner caller. One careful hub per session.

---

## Idiom session 200 — 2026-06-18 — Execution: thread the input-stack/error-recovery layer (scanner prerequisites)

**Type:** execution (workstream 01), large multi-commit threading session per
user request for a bigger session. First committed the inherited session 198/199
typed-consumer WIP as a clean baseline. Recon (three read-only Explore agents)
confirmed the typed-consumer vein is thin and that `get_next` is **not** directly
threadable — it is gated on the input-stack/error layer — so the session
threaded that whole layer, the documented `get_next` prerequisite.

**Code (6 commits):**
1. **input-stack token-list layer** — `end_token_list`/`begin_token_list`/
   `end_file_reading`/`back_input` → `&mut EngineState` `_in` cores. Input/param/
   align_state reads move to direct `st.input.*`/`st.scanner.*`; `cur_input`/
   `cur_tok`/`first`/`line` stay raw `mod raw` (safe under the borrow); list
   recycling/avail-pop go through threaded `flush_list`/`delete_token_ref`/
   `get_avail`; `begin_token_list`'s `\tracingmacros` diagnostic prints in-order
   via the threaded `print_*_in`/`token_show_in`/`*_diagnostic_in` cores. New
   `InputAbort` sentinel (§5) defers cold `overflow`/`fatal_error`/arena-
   exhaustion; extracted `mem::fire_main_memory_overflow`.
2. **begin_file_reading + pseudo_input** threaded (input/buffer/pseudo-file
   state direct; `cur_boundary`/`cond_ptr` from `st.save_stack`/`st.scanner`).
3. **runaway** → `runaway_in` (self-contained cold diagnostic, no abort path).
4. **print_spec_in** typed glue-spec accessors (`NodePtr<GlueSpecNode>`).
5. **back_error/ins_error/insert_relax** threaded (build on `back_input_in`/
   `error_in`; dual `Result<ErrorOutcome, InputAbort>` shape).
6. **check_outer_validity** → `check_outer_validity_in` — the capstone; the
   whole runaway/forbidden-cs/incomplete-`\if` recovery runs under one borrow,
   calling the now-threaded `begin_token_list_in`/`runaway_in`/`ins_error_in`/
   `error_in`/print cores; terminal `ErrorOutcome` returned for the wrapper.

This clears **every** `with_state`-bearing blocker between `get_next` and a
single-borrow `get_next_in` (id_lookup/file_warning/error were already threaded;
the input/error layer is now too). Deleted now-dead globals `line_stack`/
`max_in_stack`/`set_line_stack`/`skip_line`.

**Tests:** `cargo check`; dashboard **29/2/7/4** after each commit (TRIP gates
the recovery paths); `cargo build --release -p tectonic`; full corpus **637
passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/
`1905.00148v1`, no NOXDV/NOLOG. Metrics: warnings **41** (held); `unsafe fn`
**525 → 527** (+2 cold-abort fire helpers `InputAbort::fire`/
`fire_main_memory_overflow`); `with_state(_mut)` sites **1083 → 1093** (+10
transient bridge-down wrappers for the 11 newly-threaded leaves — retire when
`get_next`/`expand`/`macro_call` thread and call the `_in` cores directly).

**Next:** thread `get_next` itself into `get_next_in(&mut EngineState)` — the
last input-layer blocker is `input_line` (io.rs); after that the ~620-line
tokenizer body is a single-borrow port calling the now-threaded `_in` cores
(`begin_token_list_in`/`end_token_list_in`/`end_file_reading_in`/
`check_outer_validity_in`/`pseudo_input_in`/`id_lookup`). One careful hub;
then `get_token`/`get_x_token`/`scan_toks` and the shipout hub open up.

---

## Idiom session 199 — 2026-06-18 — Execution: typed split-top/pagebuilder/main-control consumers

**Type:** execution (workstream 01/01c), low-docs typed-consumer continuation
with two read-only subagents scouting scanner and remaining small typed slices.

**Code:** Took the safe typed cleanup vein rather than forcing the full scanner
hub. In `packaging.rs`, `insert_split_top_skip_before` now treats `temp_ptr()`
after `new_skip_param` as a typed `GlueSpecNode` and updates its width through
`GlueSpecNode` accessors; `prune_page_top` discards glue/kern/penalty nodes via
a small typed `PruneDiscardNode` helper using header `link`/`set_link` instead
of raw `b32s1` link reads/writes. In `pagebuilder.rs`, `update_last_node_info`
uses `glue.glue_spec()` directly. In `main_control.rs`, freshly allocated
append-glue/append-kern nodes are reclassified with `classify_glue`/
`classify_kern` instead of local unchecked constructors. Scanner scout found
the real next slice is still `get_next`/control-sequence lookup with deferred
outcomes around `check_outer_validity`, input EOF helpers, token-list insertion,
diagnostics, and fatal errors.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`./plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (script exit 1 due to
those expected divergences). Warnings held at **41**; `unsafe fn` **525**;
`with_state(_mut)` sites **1014**.

**Next:** stop mining `packaging` unless a tiny typed consumer is obvious; the
next meaningful progress is the deliberate scanner/token hub. Start with
`get_next` control-sequence lookup and an outcome enum that defers
`check_outer_validity`, `begin_token_list`/`end_*`, `pseudo_input`/`input_line`,
file warnings, fatal errors, and hash/pool/string overflows until after the
borrow drops.

---

## Idiom session 198 — 2026-06-18 — Execution: packaging typed cursor/probe cleanup

**Type:** execution (workstream 01/01c), low-docs typed-consumer batch with two
read-only subagents scouting `packaging` targets.

**Code:** Tightened `packaging.rs` typed boundaries without changing borrow
shape. `vert_break` now carries nullable cursor/list pointers as `MemPtr`; the
glue/kern update decision carries typed `VertBreakUpdateSource` handles instead
of rechecking the raw `GLUE_NODE..=KERN_NODE` range; and height updates use
`VertBreakHeightSource::{Kern,GlueSpec}` rather than raw `q + 1` reads.
`split_vlist_at_break` classifies split marks through `AnyNode::Mark` instead
of unchecked construction. `hpack_in` native-word discretionary probes now use a
guarded `classify_disc_hp` helper and typed `DiscNode` subtype/link reads,
preserving the synthetic `r + 5` guard and hi-mem partition.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (script exit 1 due to
those expected divergences). Warnings held at **41**; `unsafe fn` **525**;
`with_state(_mut)` sites **1014**.

**Next:** cheap `packaging` typed-consumer cleanup is thinner now. Possible
small follow-up: `prune_page_top`/split-top height wrapper, but preserve
`RuleDimen::raw()` for running rule dimensions. Otherwise take the deliberate
scanner hub slice; broad build-page/shipout hubs still depend on scanner/token
threading.

---

## Idiom session 197 — 2026-06-18 — Execution: mlist/pagebuilder typed boundary cleanup

**Type:** execution (workstream 01/01c), larger low-docs typed-consumer
continuation with read-only subagents.

**Code:** Continued from the session-196 dirty WIP and validated the whole
batch. Added the remaining small typed-boundary cleanup around `mlist_to_hlist`:
`math_kern_global` now takes `NodePtr<KernNode>`, the `CHOICE_NODE`/`RULE_NODE`/
`GLUE_NODE`/`KERN_NODE` first-pass arms classify through `MemState` instead of
locally constructing unchecked handles, conditional-math-glue lookahead uses
`AnyNode`, and the second-pass passthrough move uses a typed `AnyNode` helper.
Retyped `mlist_track_box` to accept `AnyBox` and added a typed rule-height/depth
tracker. In `pagebuilder`, `ensure_vbox_is_hbox` now uses `AnyBox::HList`, and
the last-node update block uses `AnyNode::{Glue,Penalty,Kern}` plus typed
glue-spec refcount/penalty/kern access under one short borrow after the old
`last_glue` ref is deleted.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Warnings held at
**41**; `unsafe fn` **525**; `with_state(_mut)` sites **1014**.

**Next:** the small typed-consumer vein is thinner now. Reasonable next slices:
`packaging` `vert_break` typed height-source cleanup, native-word disc probes in
`hpack_in` (higher corpus risk), or a deliberate scanner hub slice. Avoid broad
`build_page`/shipout hub threading until scanner/token helpers are ready.

---

## Idiom session 196 — 2026-06-18 — Execution: larger typed consumer batch

**Type:** execution (workstream 01/01c), low-docs typed-consumer batch with
read-only subagents.

**Code:** Retyped the small main-control appenders (`append_penalty`,
`append_glue`, `append_kern`) around the built penalty/glue/kern families while
keeping scans and pagebuilder calls outside the borrow. Retyped the math
script-stack helpers so `make_scripts_global` and its script-kern/subsup helpers
carry `NodePtr<Noad>` and `AnyBox` handles instead of repeatedly reclassifying
raw `i32` noads/boxes; mixed/null nucleus list heads stay raw. Retyped
`packaging::collect_split_mark` to take `NodePtr<MarkNode>` at the checked
`MARK_NODE` branch.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Warnings held at
**41**; `unsafe fn` **525**; `with_state(_mut)` sites **1011**.

**Next:** continue the math typed-consumer seam around `mlist_to_hlist`
(`mlist_choice_node`, `mlist_track_box`, rule/kern arms) or take another small
typed/threaded island such as `pagebuilder`'s last-node update block. The
scanner hub remains the larger deliberate slice.

---

## Idiom session 195 — 2026-06-18 — Execution: main-control inter-character action core

**Type:** execution (workstream 01), low-docs continuation. Took the smaller
main-control slice identified in session 194 instead of forcing the scanner hub.

**Code:** Extracted the duplicated XeTeX inter-character token insertion logic
in `main_control.rs` into an `InterCharInputAction` that probes
`\XeTeXinterchartoks` under a short `&mut EngineState` borrow, then performs
`back_input`/`begin_token_list` after the borrow drops. Applied it to
`main_control_hmode_inter_char_check`, normal-space handling, native collection
prefix/suffix checks, regular-font entry, and lookahead. The helper uses the
threaded `save::find_sa_element(&mut mem, &mut scanner, ..., false)` path; token
stack mutation stays deferred.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Warnings held at
**41**; `unsafe fn` **525**; `with_state(_mut)` sites **1007**.

**Next:** scanner remains the deliberate large hub (`id_lookup`,
`check_outer_validity`, input/EOF helpers, diagnostics, fatal sentinels). If
staying smaller, continue `main_control` by finding more duplicated short-borrow
probe/action islands without widening input-stack borrows.

---

## Idiom session 194 — 2026-06-18 — Execution: typed math rule/kern/box consumers + linebreak glue/math handles

**Type:** execution (workstream 01/01c), low-docs mechanical typing batch with
read-only subagents. Scanner recon confirmed `get_next`/`get_token` remains a
large hub, and even `scan_toks` cannot hold a single borrow across legacy token
callees. Main-control recon confirmed the smaller inter-character action-enum
slice is viable, but this session stayed on already-threaded typed consumers.

**Code:** Retyped `math.rs` rule/kern/box consumers without widening borrow
lifetimes: `fraction_rule` now returns `NodePtr<RuleNode>`; fraction/overbar
linking uses typed `KernNode`/`RuleNode` handles; `make_under_prepare`,
`zero_box_width`, `stack_into_box`, `stack_glyph_into_box`,
`native_glyph_vbox_finish`, and `stack_glue_into_box` take `AnyBox` at their
local boundaries. Native glyph/whatsit payloads stay raw because that typed
family is not built. In `linebreak.rs`, `finish_last_line_fit` takes the
`GlueNode` handle created by `new_param_glue`, and LR-stack update helpers take
`NodePtr<MathNode>` only after existing `MATH_NODE` guards.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (doc-count 639).
Warnings held at **41**; `unsafe fn` **525**; `with_state(_mut)` sites **1001**.

**Next:** the scanner hub still needs a deliberate multi-helper slice
(`id_lookup`, `check_outer_validity`, input/EOF helpers, diagnostics, fatal
sentinels). The smaller next candidate is main-control inter-character spacing:
extract an action core for the `back_input`/`begin_token_list` deferral, starting
with `main_control_hmode_inter_char_check`.

---

## Idiom session 193 — 2026-06-18 — Execution: small format helper thread + Noad typed cleanup

**Type:** execution (workstream 01/01c), low-docs batch with read-only
subagents. Code kept to two safe slices: (1) threaded `engine::sort_avail` to
`sort_avail_in(&mut MemState)` and changed `store_fmt_file` to call it through
its own short borrow. This removes one of session-192's `store_fmt_file`
blockers without trying to single-borrow the dump loop. (2) Retyped several
already-threaded math noad helpers to take `NodePtr<Noad>` instead of raw `i32`:
`make_over`, `make_under_finish`, `make_vcenter`, `make_radical_finish`,
`make_fraction_finish`, `make_ord_in`, and `append_script_box`; the unchecked
raw→typed conversion now stays at the existing wrapper boundary via
`assume_noad(q)`. No packaging/native-math borrow lifetimes were widened.

**Recon:** scanner scout says the real next scanner slice is `get_next_in` +
`get_token_in`, but it is a large hub: `id_lookup`, `check_outer_validity`,
input-line/EOF helpers, diagnostics, and fatal sentinels must be handled first.
Main-control scout says a viable non-scanner next slice is the inter-character
spacing path (`main_control_hmode_inter_char_check` +
`main_control_append_normal_space`) using an action enum to defer
`back_input`/`begin_token_list`. Shipout hub remains scanner-gated.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` returned **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (doc-count 639).
Warnings held at **41**; `unsafe fn` **526 → 525**; `with_state(_mut)` sites
currently **1001**.

**Next:** take the scanner hub deliberately (`get_next_in` + `get_token_in` with
sentinels), or the smaller main-control inter-character spacing action-core
slice. `store_fmt_file` still has `make_string_global`/`pack_job_name`/
`make_name_string` plus print/dump interleaving; do not force it into one borrow.

---

## Idiom session 192 — 2026-06-17 — Execution: thread `load_fmt_file` to a single-borrow core

**Type:** execution (workstream 01). Took `format_io.rs` — the second-largest
non-`globals` `with_state` host (60 sites) that prior sessions had not touched.
**Recon split:** `load_fmt_file` is the clean win (no printing, and its only
interleaved non-state calls are the `undump_*` file FFI, which are NOT
`with_state`-bearing), so it threads to **one** `&mut EngineState` borrow held
across the entire format undump. `store_fmt_file` is **not** — its dump loops are
interspersed with six `with_state`-bearing helpers (`make_string_global`,
`pack_job_name`, `make_name_string`, `sort_avail`, `pseudo_close_global`,
`init_trie`) plus `print_*` that need the borrow dropped, so a single borrow
can't span it (bucket-D); deferred.

**Code (1 commit):** carved `load_fmt_file_in(st: &mut EngineState, fmt_in) ->
Result<bool, *const c_char>` out of the ~720-line driver. The thin wrapper keeps
the file-open prelude outside the borrow (`name_of_file`/`pack_buffered_name` are
still `with_state`-bearing), opens `fmt_in`, then runs the core under one
`with_state_mut`. Inside, every `crate::globals::X()`/`set_X` accessor and ~40
`with_state(_mut)` islands (Vec (re)allocations + base-pointer grabs) became
direct `st.<cluster>.<field>` access / `st.mem.b32s0` / `st.eqtb.set_b16s1` /
`st.eqtb.word`, with the `mem`/`eqtb`/`hash` base pointers grabbed once after
their `Vec` is allocated (raw ptr, no held borrow). The ~30 cold
`bad_format_file()`/`_tt_abort(msg)` paths ride an **abort sentinel** (`Err(msg)`
returned; the wrapper fires `_tt_abort(msg)` after the borrow drops, `01b` §5) —
`bad_format_file` itself is now dead and deleted. Raw `mod raw` accessors
(`cur_input().loc`, `set_cur_input_loc`, `set_cur_list_head/tail`) stay as global
calls (safe under the borrow). Deleted 3 now-dead `globals.rs` accessors
(`set_trie_max`, `trie_size`, `set_op_start_entry`) + 39 now-unused format_io
imports.

**Tests:** `cargo check`; `cargo fmt`; dashboard **29/2/7/4** (TRIP exercises a
real `\dump` **and** format-load, so it directly gates this slice); rebuilt
`target/release/tectonic`; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count ==
baseline, no NOXDV/NOLOG. Metrics: warnings **41** (held); `unsafe fn` net-neutral
(−1 `bad_format_file`, +1 `load_fmt_file_in`); `with_state(_mut)` sites
**1037 → 1000** (−37: format_io 60→27, all 27 remaining in `store_fmt_file`).

**Next:** `store_fmt_file` is the only `with_state` host left in `format_io`
(27 sites) but is scanner-/helper-gated (the six bucket-D helpers above + native-
font recoverable `error()`); threading it needs those helpers threaded first, or
a scoped-island treatment that won't collapse much — low priority. The big
remaining levers are unchanged: the **scanner** (`get_next`/`get_x_token`/
`scan_toks`/`get_token`) which gates the shipout hub + `main_control` handlers +
`output.rs` `show_whatever`/`issue_message`, and the `math.rs` `make_*_global`
sub-drivers of `mlist_to_hlist` (bucket-D scoped islands). One careful hub/host
per session; the mechanical leaf phase stays drained.

---

## Idiom session 191 — 2026-06-14 — Execution: thread the scanner-free shipout leaves + build the threaded DVI-emit layer

**Type:** execution (workstream 01). Took the session-190 "Next" (the shipout
hub: `ship_out`/`hlist_out`/`vlist_out`). **Recon finding that reshaped the
slice:** the hub triple is *mutually recursive* (`hlist_out`↔`vlist_out`↔
`output_hlist_glue`) and, more importantly, gated on the **scanner** — its
whatsit arm calls `out_what`, which calls `write_out`, which runs
`scan_toks`/`get_token`. So a single-borrow hub is not reachable until the
scanner threads (the "big coupling cluster"). The DVI emit path, by contrast,
is *not* a blocker: `write_to_dvi` only re-enters `with_state` to read
`shipout.dvi_file`/`dvi_buf`, which under a held `&mut ShipoutState` are direct
field reads. So this session threaded the **scanner-free / DVI-free shipout
leaves** and built the reusable threaded DVI-emit layer the future hub needs.

**Code (4 commits):**
1. `reverse(mem, shipout, scanner, fonts, …) -> Result<i32, ShipoutAbort>` —
   does no DVI output (pure list-reverse + width accounting); the only cold path
   is `confusion("LR2")`, deferred via the new `ShipoutAbort` sentinel. `LR_ptr`/
   `LR_problems`/`cur_dir` are `scanner` fields; `cur_h`/`rule_wd` are `shipout`.
2. `merge_justifiable_native_words(mem, strings, fonts, eqtb, …) -> Result<(),
   ShipoutAbort>` — native-word coalescing; string-pool writes are direct,
   `skip_ignorable_interword_nodes`/`is_ignorable_interword_node` called under
   the borrow, cold `overflow("pool size")` + `new_native_word_node`'s
   `NodeRecycle` + `flush_node_list`'s `Confusion` ride the sentinel. Deleted
   dead `store_justified_native_glyphs_global` + global `font_letter_space_entry`.
3. **Threaded DVI-emit layer** (the prerequisite infra for the hub):
   `write_to_dvi(&ShipoutState,…)`, `handle_dvi_flush`, `emit_dvi_out`,
   `emit_dvi_four`, `emit_movement(&mut Mem,&mut Shipout,…)` — each performs the
   buffer flush by reading `shipout` fields directly and defers only the truly
   cold aborts (`DviWriteFailed`/`DviLengthOverflow`/`MainMemoryOverflow`).
   Threaded `pic_out_in(&mut EngineState,…)` + `special_out_in(…)` on it (both
   print via the session-182 `print_*_in`/`show_token_list_in` cores; reached
   only through the still-global `out_what`, so they keep thin bridge wrappers).
   Deleted dead global `print_raw_char`.

**`ShipoutAbort` sentinel (§5):** `ConfusionLr2`/`PoolSizeOverflow`/`NodeRecycle`/
`Confusion`/`DviWriteFailed`/`DviLengthOverflow`/`MainMemoryOverflow`, fired by
`fire_shipout_abort` after the borrow drops.

**Tests:** `cargo fmt`; `cargo check`; dashboard **29/2/7/4**; rebuilt
`target/release/tectonic`; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1`, doc-count 639 ==
baseline, no NOXDV/NOLOG. Metrics: warnings held; `unsafe fn` **526 → 525**;
`with_state(_mut)` sites **1040 → 1037** (leaves still carry transient bridge
wrappers that retire when the hubs thread).

**Next:** the shipout hub triple (`hlist_out`/`vlist_out`/`ship_out` +
`output_hlist_glue`) is **scanner-gated** via `out_what`→`write_out` — thread
the scanner (`get_token`/`get_x_token`/`scan_toks`) first, then the hub becomes a
single-borrow pass over the now-threaded DVI-emit layer + `reverse`/`pic_out`/
`special_out`/`out_what` cores. Otherwise pick a non-shipout hub (`mlist_to_hlist`
remains scoped-island form; its `make_*_global` sub-drivers are unthreaded). The
threaded DVI-emit layer (`emit_dvi_*`/`emit_movement`) is the reusable substrate
for that future hub slice.

---

## Idiom session 190 — 2026-06-14 — Execution: thread `line_break` itself (the line-breaker hub)

**Type:** execution (workstream 01). Large minimal-docs hub slice — the
session-189 "Next" item. Threaded the ~660-line `line_break` driver into
`line_break_in(&mut EngineState) -> LineBreakOutcome`; `line_break(d)` is now a
thin `with_state_mut` wrapper that dispatches the deferred cold abort.

**Code:** the core holds one borrow across the whole pass and reads
`mem`/`eqtb`/`scanner`/`linebreak`/`hyph`/`fonts` directly, calling the
already-threaded cores under that borrow (`start_linebreak_pass`, `try_break`
via the `line_break_try_break` split-borrow shim, `finite_shrink`,
`hyphenate_in`, `split_native_tail_for_linebreak`, `choose_best_break`,
`finish_last_line_fit`, `post_line_break_in`, `init_trie_in`,
`set_current_hyph_index`). `cur_list` is the deferred `mod raw` static, so its
reads/writes are raw and don't re-enter `with_state`. The `add_active_width`
closure became an `aw!` temp-then-`+=` macro; the nested `add_node_width_to_active`
became a free fn over `(mem, fonts, scanner)`. Cold aborts ride a new
`LineBreakOutcome` sentinel (`Overflow`/`Node`/`TryBreak`/`Hyphenate`/`Trie`/
`PostLineBreak`/`Confusion`/`Error`). New helpers: `mem::pop_nest_in(mem,input)`
+ `globals::set_cur_list_record`, `scan::max_hyphenatable_length_in(eqtb)`; made
`hyphenation::{init_trie_in,TrieAbort,trie_abort}` `pub(crate)`.

**Regression caught + fixed mid-session (TRIP):** the first cut deleted
`finite_shrink_global`, which carried the *recoverable* "Infinite glue shrinkage
found in a paragraph" diagnostic (`error()` that **returns**, not a `-> !`
abort) — the threaded `finite_shrink` core only makes the shrink finite, never
prints. The four `tex-outputs`/`engine_rust`/`latex_goldens` suites stayed green;
only **TRIP** caught it (1/2: the message + 5 help lines missing from
`trip.log`). Fixed with `finite_shrink_emit(st,p)`, which (when
`no_shrink_error_yet`) emits the diagnostic through the **threaded**
`error_here_with_diagnostic_in` / `capture_to_diagnostic_in` / `error_in` path
under the borrow, then calls `finite_shrink`; a non-`Continue` `ErrorOutcome` is
deferred to the wrapper via `LineBreakOutcome::Error`. **Lesson: a deleted
`_global` wrapper may carry an inline recoverable-`error()` diagnostic the
threaded core lacks — grep the old wrapper body for `error()`/`print_*`/help
lines before deleting, and lean on TRIP (the only suite exercising the infinite-
shrinkage path).**

**Cleanup:** deleted 8 now-dead `_global` wrappers (`try_break_global`,
`start_linebreak_pass_global`, `choose_best_break_global`,
`finish_last_line_fit_global`, `free_active_and_passive_lists_global`,
`free_final_linebreak_lists_global`, `finite_shrink_global`,
`split_native_tail_for_linebreak_global`, `set_current_hyph_index_global`) and
~79 orphaned bridge accessors across `linebreak.rs` (`node_type`/`box_width`/
`glue_*`/`ligature_*`/etc.) and `globals.rs` (the linebreak/hyph scalar
getters/setters: `active_width`/`background`/`cur_p`/`ha`/`hf`/`hn`/`threshold`/…).

**Tests:** `cargo fmt --check`; `cargo check`; dashboard **29/2/7/4** (TRIP back
to 2/2 after the fix); release build; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` / `1905.00148v1`, doc-count 639 ==
baseline, no NOXDV/NOLOG. Metrics: warnings **41** (held); `unsafe fn`
**533 → 526**; `with_state(_mut)` sites **1125 → 1040** (−85, the bridge-accessor
collapse).

**Next:** thread the next recursive hub — `mlist_to_hlist` or `hlist_out`/
`vlist_out`/`ship_out` (their callees are largely threaded; native/OT measurer
is now re-entry-clean). `get_next`/`expand`/`macro_call`/`main_control` and the
scanner remain the big un-threaded coupling cluster. Per recon the mechanical
leaf phase stays drained — one careful hub per session.

---

## Idiom session 189 — 2026-06-14 — Execution: thread `hyphenate` core

**Type:** execution (workstream 01). Minimal-docs batch; used one read-only explorer
for hazard scouting. Finished the session-188 handoff item by carving
`hyphenate_in(&mut EngineState)` out of the legacy global `hyphenate()` wrapper.

**Code:** `hyphenate()` is now a thin `with_state_mut` wrapper. The new core reads
hyphen exceptions/pattern tries via `st.hyph` + `st.strings`, calls
`hyphenate_native_word_if_needed`, `reconstitute`, `new_character_in`,
`flush_node_list`, and `flush_list` directly under the borrow, and defers cold
abort tails through `HyphenateAbort`. Removed the now-dead `reconstitute_global`
bridge and dead hyphenation getters from `globals.rs`; added narrow
`FontState::font_bchar_entry`.

**Tests:** `cargo fmt --check`; `cargo check -p tectonic_engine_xetex`; dashboard
**29/2/7/4**; release build; full corpus **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` / `1905.00148v1`, no NOXDV/NOLOG.
Metrics: warnings **41** (held); `unsafe fn` **533**; `with_state(_mut)` sites
**1125**.

**Next:** thread `line_break_in`. Remaining blockers are in the outer
`line_break` body: `pop_nest`, finite-shrink/error sentinel handling, and the
large accessor/global-helper collapse onto the already-threaded helpers
(`hyphenate_in`, `try_break`, `post_line_break_in`, `start_linebreak_pass`,
`choose_best_break`, `finish_last_line_fit`, `split_native_tail_for_linebreak`).

---

## Idiom session 188 — 2026-06-14 — Execution: reduce native/hyphenation bridges on the path to `line_break_in`

**Type:** execution (workstream 01). Per user request, kept docs minimal and used
subagents only for bounded side work. Explorer confirmed `line_break_in` is still
blocked by unthreaded `hyphenate()` plus `pop_nest`/finite-shrink sentinels; a
worker threaded `main_control::new_character`.

**Code:** `new_character_in(&mut EngineState, f, c) -> Result<i32, NewCharacterAbort>`
now handles native chars via `new_native_character_in`, classic chars via
`get_avail` + font fields, and `char_warning_in`; the legacy wrapper fires cold
aborts after the borrow drops. In `linebreak.rs`, collapsed the line-break native
word split/re-measure sequence into `split_native_tail_for_linebreak(st, start,
advance_ha)` and deleted dead `set_native_node_len`/`copy_native_tail_global`/
`set_native_metrics_global` bridges. Extracted `hyphenate_native_word_if_needed(st,
hn_v)` so the hardest native branch is reusable by a future `hyphenate_in`.

**Tests:** `cargo check -p tectonic_engine_xetex`; `fmt`; dashboard **29/2/7/4**;
release build; full corpus **637 passed / 2 diverged / 0 missing-golden**, only
known `1905.00073v1` / `1905.00148v1`, no NOXDV/NOLOG. Metrics: warnings **41**
(held); `unsafe fn` **535** (held); `with_state(_mut)` sites **1140**.

**Next:** finish `hyphenate_in` (now mostly mechanical state-field replacement,
with `get_node`, `new_character_in`, `reconstitute`, and list flush/free sentinels),
then thread `line_break_in`. `pop_nest` still needs a threaded core before the
outer driver can hold one borrow across the whole function.

---

## Idiom session 187 — 2026-06-14 — Execution: break the native-font measurer FFI re-entry wall + thread the native-word hyphenation island

**Type:** execution (workstream 01). Took session 186's recommended highest-leverage
slice — the native-font measurer (the "135/136 FFI re-entry wall" blocking
`line_break_in`/`mlist_to_hlist`/`hlist_out`). **One coherent commit** (`803be361`).

**What the wall actually is (corrected understanding):** the C++ `measure_native_node`
is *already* threaded (`&FontState`, re-entry-free). The real barriers are the
**Rust** helpers on the native-char build path that re-enter `with_state`:
`set_native_metrics` (called `measure_native_node_global`), `apply_mapping` (manages
the `mapped_text` Vec via `with_state_mut`), and `new_native_character` (string-pool
+ apply_mapping + char_warning + build island + measurer). Note RefCell allows
*nested shared* borrows, which is why the legacy chain (`with_state(shared)` →
inner `with_state(shared)`) never panicked — but a threaded caller holding
`with_state_mut` makes any inner `with_state` panic. That is the wall.

**Code:**
- `set_native_metrics(&mem,&eqtb,&fonts,p)` calls threaded `measure_native_node`
  directly.
- `apply_mapping(&mut FontState, …)` — threaded (only touches `mapped_text`);
  `main_control`'s caller evaluates `native_text`/`native_len` before the borrow.
- `new_native_character_in(&mut EngineState, f, c) -> Result<i32, NativeCharAbort>`
  + thin wrapper. All staging/mapping/`char_warning_in`/build/measure under one
  borrow. Cold aborts ride `NativeCharAbort` (every non-`Continue` `ErrorOutcome`
  is divergent in `finish()`, so deferral == the legacy global `char_warning`
  abort; `finish()` is a safe fn with narrow `unsafe`).
- `hyphenate`'s `goto_found` native_word_node split → one `with_state_mut` island
  over the threaded `new_native_word_node`/`set_native_metrics`/`new_disc`/
  `new_native_character_in`/`flush_node_list` cores, original allocation order
  preserved. Cold aborts ride `HyphNativeAbort` (`01b` §5).

**Regression caught and fixed mid-session (important lesson):** the first cut
threaded `new_native_character_in` but `apply_mapping` *also* re-enters
`with_state` — so the wrapper panicked (`RefCell already borrowed`) on every
mapping/native-font doc. **The four-suite dashboard did NOT catch it** (its fonts
don't use TECkit mappings); only the full corpus did — as **NOXDV/NOLOG** (no
output, ~49 docs), which the PARITY pass/diverge counts hide. Lesson: after any
native-font / measurer slice, diff the corpus *doc count* (passed+diverged) vs
baseline, not just the divergence list; a drop = crashes. Resolved by threading
`apply_mapping`. (`testfiles/github-0081` showed a one-off DIFF in one corpus run
but is deterministic + golden-matching standalone and doesn't hyphenate — a
harness flake, not a regression.)

**Tests:** `cargo check`+`fmt`; dashboard **29/2/7/4**; rebuilt
`target/release/tectonic`; `corpus.sh check all` = **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1`/`1905.00148v1` (verified == the
session-186 baseline binary on this machine, incl. NOXDV/NOLOG = 0). Metrics:
warnings **41** (held); `unsafe fn` **536** (held); `with_state(_mut)` sites
**1141 → 1144** (+3 transitional scoped islands, retire when `line_break`/
`hyphenate` fully thread, cf. session 136's accepted +2).

**Next:** the native-char/measurer leaves are now re-entry-clean, so the path is
open to thread `line_break` itself (its two big callees `post_line_break`/
`try_break` were done in ≤186; the remaining `set_native_metrics_global`/
`new_native_character` call sites in its main loop can now pass borrows through),
then `mlist_to_hlist` / `hlist_out`. Each is still a careful one-hub slice. **Re
the broader pace: the mechanical leaf phase is genuinely drained (confirmed via
`phase_c_recon.py` — top items are unit tests + the intentionally-deferred
hot-tokenizer accessors + 3 trivial dead wrappers); remaining work is
hub-by-hub, subagents don't help, and corpus doc-count must be checked, not just
the divergence list.**

---

## Idiom session 186 — 2026-06-14 — Execution: thread the line-breaker driver tail (post_line_break + hpack/vpack diag helpers)

**Type:** execution (workstream 01). **Recon finding worth recording up front:**
the cheap mechanical *leaf* phase is essentially complete — every module is
threaded down to `_in`/core fns + thin `_global`/`with_state` bridge stubs. The
~1140 remaining `with_state` sites are (a) the `globals.rs` accessor bridge,
(b) bridge stubs waiting on their callers, and (c) a small set of un-threaded
**recursive top-level hubs**: `mlist_to_hlist`, `hlist_out`/`vlist_out`/
`ship_out`, `line_break`, `get_next`/`expand`/`macro_call`, `main_control`.
There is **no cheap keystone left with a leaf fan-out** (182–185 drained those).
Progress now = threading a hub, which is bucket-C/D: each is 500–2000 lines,
recursive, and interleaves state mutation with **with_state-bearing diagnostic
printing whose byte order is parity-critical** — so a single-borrow thread must
print via the session-182 `print_*_in` cores, never defer-and-reorder. Most hubs
(`line_break`/`mlist_to_hlist`/`hlist_out`) also hit the **native-font / OT-math
measurer** (the 135/136 FFI-re-entry wall). One hub per session, real regression
risk; subagents don't help (per-file edit serialization + hazard judgment). This
is the honest pace from here.

Took the most tractable hub vein: the **line-breaker driver**. Its two big
callees were the blockers — `try_break` was found **already fully threaded**
(`&mut st.mem/scanner/linebreak/eqtb/fonts/config/errors`), so only
`post_line_break` remained, and it is **FFI-free** (only cold aborts).

**Code (2 commits):**
1. **hpack/vpack diagnostic helpers** — split `print_{hpack,vpack}_diag` +
   `print_{hpack,vpack}_common_ending` into thin `with_state` wrappers +
   `pub(crate)` `*_in` cores that print under a live `&mut EngineState` borrow
   (via the `print_*_in`/`begin_diagnostic_in`/`show_box_in`/`short_display_in`
   cores). `output_active`/`pack_begin_line`/`font_in_short_display` are now
   field-direct (`st.shipout`/`st.scanner`/`st.errors`); `line()` stays raw.
   `PackDiag` → `pub(crate)`. Retired now-dead global wrappers `short_display`,
   `set_font_in_short_display`, `pack_begin_line`.
2. **post_line_break** — collapsed from scoped-island form (3 `with_state_mut`
   per line) to a single-borrow `post_line_break_in` held across the whole
   per-line loop. The per-line `hpack` + its in-order over/underfull-`\hbox`
   diagnostic now run via a new `packaging::hpack_emit_in(st,…) ->
   Result<i32, NodeRecycleAbort>` (= `hpack_in` + `print_hpack_diag_in` under the
   borrow, abort returned not fired). Cold aborts (`node_recycle_abort`/
   `pdf_error`/`confusion`) ride a new `PostLineBreakOutcome` sentinel fired by
   the thin wrapper after the borrow drops (§5). `cur_list_*` stay raw.

**Tests:** `cargo check` + `fmt --check`; dashboard **29/2/7/4** after each
commit; rebuilt `target/release/tectonic`; `plan/scripts/corpus.sh check all`:
**637 passed / 2 diverged / 0 missing-golden**, only the known FP divergences
`1905.00073v1` / `1905.00148v1`. Metrics: warnings **41** (held); `unsafe fn`
**536 → 535**; `with_state(_mut)` sites **1146 → 1141**.

**Next:** the line breaker's two big callees (`post_line_break`, `try_break`)
are now both threaded, so the only thing between `line_break` and a single-borrow
`line_break_in` is **its own hyphenation pass's native-font-measurer FFI**
(`set_native_metrics`/`new_native_word_node`/`copy_native_tail` — the 135/136
wall). Threading that measurer is the highest-leverage next slice (it also
unblocks `mlist_to_hlist` and `hlist_out`); it needs the deferred-outcome /
scoped-island treatment for the C++ measurer re-entry. Otherwise pick another
hub. Do **not** expect mechanical fan-out — it's careful one-hub work now.

---

## Idiom session 185 — 2026-06-14 — Execution: thread the `error()` keystone + the errors.rs helper cluster + char_warning/change_if_limit

**Type:** execution (workstream 01 / fused `01b`+`01c`). Large low-docs batch
per user request. Took the documented keystone — `error()` — which gated the
entire errors.rs helper family (every helper bottoms out in it) and the
output.rs `char_warning`, then cashed it out.

**The §5 lever:** `error()`'s cold abort paths (`halt_on_error_p`, the 100th
error, plus the `confusion`/`overflow` paths reached by threaded helpers) are
`with_state`-bearing/FFI and would nest-panic under a live borrow. Resolved with
a `#[must_use] ErrorOutcome` sentinel enum (`Continue`/`Halt`/`Hundred`/
`Confusion(&CStr)`/`Overflow{what,n}`) + `ErrorOutcome::finish()`: the threaded
`_in` cores do all the printing/state writes under the borrow and *return* the
cold-abort sentinel; the thin global wrapper runs `finish()` (→
`post_error_message`/`_tt_abort`/`confusion`/`overflow`) only after the borrow
drops. Threaded helpers that call `error_in` propagate the outcome up to their
wrapper. On a passing run the sentinel is always `Continue`.

**Code (4 commits):**
1. **error keystone** — `error_in(&mut EngineState) -> ErrorOutcome` (hot path
   threaded over `errors`/`shipout`/`config` fields + `show_context_in`/
   `print_*_in`); `error_here_with_diagnostic_in` (output.rs) and
   `give_err_help_in` added. Deleted now-dead global wrappers `give_err_help`,
   `show_context`, `diagnostic_print_file_line`, `print_in_mode` and the dead
   getters `help_line`/`use_err_help`/`halt_on_error_p`.
2. **errors.rs helper cluster** — `int_error`, `not_font_error_common` +
   `not_{aat,aat_gr,ot,native}_font_error`, `mu_error`, `extra_right_brace`,
   `you_cant` (returns `()` — no `error()` call), `report_illegal_case`,
   `cs_error`, `box_error` all threaded onto `_in` cores; `box_error_in` uses
   threaded `flush_node_list(&mut st.mem,…)` and defers its `confusion` via
   `Confusion`. `privileged` left as-is (only raw `cur_list_mode` + the global
   `report_illegal_case`, holds no borrow).
3. **char_warning** — threaded onto `error_in` + threaded `make_string(&mut
   st.strings)`; the `error()` and make_string-overflow cold paths both ride the
   sentinel (`Overflow`). Deleted now-dead `print_hex`/`print_ucs_code` global
   wrappers + `gave_char_warning_help` getter.
4. **change_if_limit** — threaded; its cold `confusion("if")` (the reason
   session 184 left it raw) now rides `ErrorOutcome::Confusion`.

**Not done (recorded):** output.rs `show_whatever`/`issue_message` remain the
only un-threaded print-family fns — both genuinely need the scanner
(`get_x_token`/`the_toks`/`scan_toks`), bucket-D, not unblocked by `error_in`
alone. `pre_error_message`/`post_error_message`/`fatal_error`/`overflow`/
`confusion`/`pdf_error`/`runaway` stay global by design (they are the cold
abort/print-and-die subtree, §5).

**Tests:** `cargo check`+`fmt --check`; dashboard **29/2/7/4** after each commit
(TRIP exercises `error()` heavily — the keystone's regression gate); rebuilt
`target/release/tectonic`; `plan/scripts/corpus.sh check all`: **637 passed / 2
diverged / 0 missing-golden**, only the known FP divergences `1905.00073v1` /
`1905.00148v1`. Metrics: warnings **41** (held); `unsafe fn` **542 → 536**;
`with_state(_mut)` call sites **1140 → 1146** (transient bridge-down wrappers for
the newly-threaded leaves; retire as their scan/main_control callers thread).

**Next:** the error/warning family is now threaded. The remaining output.rs
targets (`show_whatever`/`issue_message`) and the deep error-emitter callers in
`scan.rs`/`main_control.rs` are all gated on threading the scanner
(`get_token`/`get_x_token`/`the_toks`/`scan_toks`) — that scanner slice is the
natural next lever and unblocks them together (and lets the new error `_in`
cores be called pass-through instead of via the bridge wrappers). Otherwise climb
a fresh bridge host (`pagebuilder`/`main_control` tails) per the recon.

---

## Idiom session 184 — 2026-06-14 — Execution: thread the output.rs diagnostic-warning cluster (+ font warnings)

**Type:** execution (workstream 01 / fused `01b`+`01c`). Large low-docs batch
per user request, run on the `thread_output_fn.py` transform. Cleared the entire
session-183 "Next" list (the diagnostic-warning cluster), then extended into the
font warnings.

**Code (2 commits):**
1. `b386e913` — threaded the **diagnostic capture pair**
   (`capture_to_diagnostic`, `diagnostic_print_file_line`,
   `diagnostic_begin_capture_warning_here`; `_in` cores are safe fns with narrow
   `unsafe {}` around the `ttbc/ttstub` diag FFI; the global wrappers stay for
   the ~150 `errors.rs`/`scan.rs`/`expand.rs`/… callers) and the **warning
   walkers** `group_warning`/`if_warning`/`file_warning`/`show_cur_cmd_chr`/
   `bad_utf8_warning`. Added `texmfmp::gettexstring_in(&StringsState, …)` so
   `diagnostic_print_file_line_in` resolves the source filename under the borrow
   without nest-panicking (the global `gettexstring` now delegates to it).
   Extended the `thread_output_fn.py` maps (grp/if_stack, cond_ptr/if_limit/
   cur_if/if_line/shown_mode, cur_boundary/level/group getters+setters, plus a
   new 2-arg `INDEXED_SETTERS` table for `set_grp_stack`/`set_if_stack`).
   Removed 5 now-dead global wrappers (`current_diagnostic`,
   `set_current_diagnostic`, `shown_mode`, and the `print_mode`/`print_group`
   global wrappers — their `_in` cores were the only remaining callers).
2. `98f7d2c8` — threaded `font_feature_warning` (kept its `#[no_mangle]` export
   wrapper) and `font_mapping_warning`, plus `print_utf8_str` (now a
   threaded-only `print_utf8_str_in` in `ext.rs`; global wrapper deleted). The
   `name_of_file()` byte loop became `print_name_of_file_in` iterating
   `st.input.name_of_file` directly, dropping the with_state-bearing
   `name_of_file()` nest-panic under the diagnostic borrow. Done by hand (the
   `extern "C"` / `name_of_file` shapes are outside the transform's reach).

**Hazard audit:** the threaded bodies' non-print calls (`cur_input`/`cur_cmd`/
`cur_chr`/`cur_list_mode`/`line`) are all RAW `mod raw` reads (safe under a
borrow); `gettexstring`/`name_of_file` were the two with_state re-entries and
were resolved as above. `show_cur_cmd_chr`'s `cur_if` chain walk and
`file_warning`'s save/cond restore are field-direct.

**Not done (recorded, unchanged from 183):** `char_warning`/`show_whatever`/
`issue_message` stay deferred — they call `error()`/`the_toks`/`scan_toks`
(scanner+error bucket-D). `change_if_limit` left raw (its cold `confusion()`
needs the §5 sentinel; isolated, low value).

**Tests:** `cargo check`+`fmt --check`; dashboard **29/2/7/4** after each
commit; rebuilt `target/release/tectonic`; `plan/scripts/corpus.sh check all`:
**637 passed / 2 diverged / 0 missing-golden**, only the known FP divergences
`1905.00073v1` / `1905.00148v1`. Metrics: warnings **41** (held); `unsafe fn`
**544 → 542**; `with_state(_mut)` call sites **1134 → 1140** (transient
bridge-down wrappers for the newly-threaded diagnostic leaves whose external
callers are still global — they retire as those callers thread, §3; no dead
accessor remained to offset, per the compiler).

**Next:** the output.rs print/show/diagnostic vein is now exhausted. The
remaining output.rs targets (`char_warning`, `show_whatever`, `issue_message`)
are all gated on threading `error()`/the scanner (`get_token`/`the_toks`/
`scan_toks`) — that scanner+error slice is the natural next lever and unblocks
them together. Otherwise climb a fresh bridge host (`pagebuilder`/`main_control`
tails) per the recon.

---

## Idiom session 183 — 2026-06-14 — Execution: thread the output.rs cmd/meaning/param printers + show_* walkers + diagnostic framing

**Type:** execution (workstream 01 / fused `01b`+`01c`). Large low-docs batch
per user request, run on a **mechanical Python transform** rather than
hand-editing (the session-182 "scoped transform" lesson generalized into a
reusable tool). Cleared the entire session-182 "Next" list, then extended.

**Tooling (the speed lever):** `plan/idiom/thread_output_fn.py` splits a named
`output.rs` print/show fn into a thin `with_state_mut` wrapper + a
`pub(crate) fn NAME_in(st: &mut EngineState, …)` core, rewriting the body:
print wrappers → their `_in` variants threading `st`; every `with_state`-bearing
global accessor → direct `st.<cluster>.<field>` access (data-driven maps for
getters/setters/indexed/nest/prefix forms, balanced-paren + literal/comment-aware
scanning); RAW accessors (`cur_chr`/`cur_cmd`/`line`/`cur_input`) left alone
(safe under a borrow). `plan/idiom/delete_fns.py` removes the now-dead global
wrappers brace-safely. Subagents were **not** used: everything lives in one file
(`output.rs`), so parallel editors would collide — the transform + batched
validation was the high-throughput, parity-safe path (session-163 finding).

**Code (3 commits):**
1. `e8bedb2f` — threaded `print_cmd_chr` (~850 lines, incl. retyping its
   `print_char_or_wide` closure to take `st`), `print_meaning`, `print_param`,
   `print_length_param`, and the `show_save_groups`/`show_activities`/
   `show_context` walkers. Added `x_over_n_in(&mut ErrorState, …)` (the global
   `x_over_n` silently wrote `tex_remainder`/`arith_error` via `with_state`, so
   calling it under `show_activities`'s borrow nest-panicked — caught by TRIP)
   and a `globals::cur_list()` getter for the `save_cur_list_to_nest` inline.
   Removed ~14 dead global wrappers (`print_param`/`length_param`/`sa_num`/
   `skip_param`/`style`/`totals`, `tally`/`first_count`/`trick_count`/`trick_buf`,
   `nest_head`/`mode_line`/`e_tex_aux`/`aux_b32s0`).
2. `c382c536` — threaded `print_file_line`, `begin_diagnostic`, `end_diagnostic`
   (clean: accessors + print only). These unblock the diagnostic-warning cluster
   (`group/if/file_warning`, `show_cur_cmd_chr`, `bad_utf8_warning`) for a later
   slice. Removed the now-dead `old_setting`/`set_old_setting` import.

**Hazard audit:** before validating, swept all non-print/non-accessor calls in
the threaded bodies for direct/indirect `with_state` re-entry; found and fixed
two (`x_over_n`, `length_global` → `string_pool::length(&st.strings, …)`); the
rest (`cur_chr`/`cur_cmd`/`line`/`cur_input`/`cur_list`) are RAW.

**Not done (recorded):** the warning/`show_whatever` cluster pulls in the
`capture_to_diagnostic`/`diagnostic_print_file_line` FFI machinery (many
`errors.rs` callers) and, for `char_warning`/`show_whatever`, the `error`/
`get_token`/`the_toks`/`scan_register_num` bucket-D helpers — a deliberate
separate slice, not extended here to avoid a late regression.

**Tests:** `cargo check`+`fmt --check`; dashboard **29/2/7/4** after each batch
(the TRIP nest-panic was caught + fixed before commit); rebuilt
`target/release/tectonic`; `plan/scripts/corpus.sh check all`: **637 passed / 2
diverged / 0 missing-golden**, only the known FP divergences `1905.00073v1` /
`1905.00148v1`. Metrics: warnings **41** (held); `unsafe fn` **551 → 544**;
`with_state(_mut)` call sites **1134**.

**Next:** thread the diagnostic-warning cluster as a coherent slice — first the
capture pair (`capture_to_diagnostic`, `diagnostic_print_file_line`,
`diagnostic_begin_capture_warning_here`, keeping global wrappers for the
`errors.rs` callers), then `show_cur_cmd_chr`/`bad_utf8_warning`/`group_warning`/
`if_warning`/`file_warning` (all now have their `begin/end_diagnostic_in` +
`show_context_in` callees). Leave `char_warning`/`show_whatever` until their
`error`/scanner bucket-D helpers are threaded.

---

## Idiom session 182 — 2026-06-13 — Execution: thread the print core + the output.rs show-node walkers

**Type:** execution (workstream 01 / fused `01b`+`01c`). Large low-docs batch
per user request. Took the documented keystone — the **print subsystem** — then
cashed it out across the `output.rs` show-node walkers, the single largest
mechanical vein (top of `phase_c_recon.py`: `show_*`/`print_*`, all blocked
because the print primitives were `with_state`-bearing so a walker holding a
`mem` borrow nest-panicked on any print call).

**Code (2 commits):**
1. `f0475cdd` — threaded the **print core** over `&mut EngineState`: `_in`
   cores for `print_char`/`print_raw_char`/`print_ln`/`print`, the
   cstr/nl/esc wrappers, `print_int`/`hex`/`scaled`/`roman_int`/`ucs_code`,
   `print_the_digs`, `warn_char`, `print_current_string`, touching
   `shipout`/`errors`/`strings`/`eqtb`/`config` fields directly; the global
   `unsafe fn`s became thin `with_state_mut` wrappers (behaviour-preserving by
   construction). Removed dead `doing_special`/`set_trick_buf`.
2. `9d898677` — threaded the **walkers**: `short_display`, `show_node_list`
   (+ `show_box`/`show_info`/`print_subsidiary_data`) and ~22 leaf print
   helpers (`print_native_word`, `print_cs`, `sprint_cs`, `print_file_name`,
   `print_spec`, `print_glue`, `print_rule_dimen`, `print_fam_and_char`,
   `print_delimiter`, `print_font_and_char`, `print_mark`, `show_token_list`,
   `token_show`, `print_size`, `print_write_whatsit`, `print_sa_num`,
   `print_mode`, `print_in_mode`, `print_group`, `print_skip_param`,
   `print_style`, `print_totals`). Walkers read mem/eqtb/strings/save_stack
   directly and call the print `_in` cores. Per established precedent the `_in`
   cores are **safe `fn`** with narrow `unsafe {}` only around the genuine
   FFI / raw-ptr / `CStr` ops (the de-unsafe drops `unsafe fn` rather than
   raising it). Added `MemState::gr`. `show_node_list` (480 lines) converted
   via a scoped ordered Python transform. Removed 11 now-dead global wrappers +
   `breadth_max`/`depth_threshold`/`set_*`/`font_in_short_display` accessors.
   `temp_ptr` stays a `mod raw` call (raw-safe under a borrow, §4).

**Tests:** `cargo check`+`fmt --check`; `node::` **48/48**; dashboard
**29/2/7/4** (after each commit); rebuilt `target/release/tectonic`;
`plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0
missing-golden**, only the known FP divergences `1905.00073v1` /
`1905.00148v1`. Metrics: warnings **41**; `unsafe fn` **564 → 551**.

**Next:** thread `show_activities` (top recon target, score 125; needs the
`nest_*` cluster), then `show_save_groups`/`show_context`/`print_cmd_chr`/
`print_meaning`/`print_param`/`print_length_param` — all were blocked behind
the print subsystem and are now unblocked (their callees are the `_in` cores
landed this session).

---

## Idiom session 181 — 2026-06-13 — Execution: threaded hyphenation trie-op/init/pattern insertion clusters

**Type:** execution (workstream 01 / fused threading, low-docs larger
mechanical batch). Used two explorer subagents for sidecar scouting: one
confirmed the next output/shipout work should start with a primitive print-core
threading slice; one confirmed `init_trie`/`new_patterns` were the next clean
hyphenation clusters.

**Code:** threaded `new_trie_op` into `new_trie_op_in(&mut HyphState, ...)`,
folded `init_trie` into `init_trie_in(&mut HyphState)`, and threaded the
completed-pattern word insertion plus saved-hyphen-code insertion subclusters.
Scanner/output calls and duplicate-pattern diagnostics stay outside hyph
borrows; overflow diagnostics still go through `TrieAbort`/`trie_abort` after
the borrow drops. Deleted the now-dead trie-op/trie-pack bridge accessors from
`globals.rs`; live engine/format/linebreak bridges stayed.

**Validation:** `cargo fmt --check -p tectonic_engine_xetex`; `cargo check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` 48/48;
dashboard **29/2/7/4**; `cargo build --release -p tectonic`; full corpus
`plan/scripts/corpus.sh check all` = **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Warnings
remain **41**; `unsafe fn` **566 -> 564**.

**Next:** either continue `hyphenation.rs` with short-borrow
`new_hyph_exceptions` subclusters, or take the output print-core slice before
`short_display`/`show_node_list` (`print_char`/`print_ln`/string/numeric
primitives behind wrappers).

---

## Idiom session 180 — 2026-06-13 — Execution: typed hpack native-word cleanup + threaded hyphenation trie packing

**Type:** execution (workstream 01 / fused threading+typing), larger
low-docs mechanical batch. Used two explorer subagents for scouting only:
one confirmed output/shipout walkers need a threaded print core first; the
other identified `hyphenation.rs` trie packing as the cleanest non-node slice.

**Code:** added `WhatsitNode::native_text_ptr{,_mut}` and used typed
`WhatsitNode`/`AdjustNode`/`InsNode`/`MarkNode` accessors in the already-
threaded `hpack_in` native-word merge / adjust extraction path. Threaded
`trie_node`, `compress_trie`, `trie_fix`, `first_fit`, and `trie_pack` over
`&mut HyphState`; `first_fit`/`trie_pack` return a local overflow sentinel so
`overflow("pattern memory")` still runs after the state borrow drops. Removed
dead `trie_taken`/`trie_min` bridge accessors from `globals.rs`.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex`;
`cargo test -p tectonic_engine_xetex node::` (**48/48**); dashboard
**29/2/7/4**; `cargo build --release -p tectonic`; `git diff --check`;
`plan/scripts/corpus.sh check all` = **637 passed / 2 diverged / 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Warnings
remain **41**; `unsafe fn` **568 → 566**.

**Next:** either keep going in `hyphenation.rs` with `new_trie_op` using the
same overflow-sentinel pattern, or take the print-core slice before
`short_display`/`show_node_list`. Avoid `shipout` walkers until the print/DVI
bridges are threaded.

---

## Idiom session 179 — 2026-06-13 — Execution: type the last untyped list-node families (Mark/Adjust/Ins/Unset + eTeX sparse-array marks)

**Type:** execution (workstream 01 / `01c` Stage 3 — the typing backlog, not
threading). Larger low-docs session per user request; 2 code commits.

**Recon finding (why this shape):** the cheap typed-*consumer* vein in
already-threaded math/packaging fns is exhausted, and the remaining math.rs
`mem_*` mass is bucket-C/D (`init_math`/`app_display`/`after_math` are
scanner/output-coupled; `clean_box`/`make_fraction_global` interleave
`mlist_to_hlist`/`hpack`/OT-math globals). The genuinely-mechanical,
compiler-checked work left is **building typed surfaces for the families
`AnyNode` still carried as raw `MemIdx`** (`01c` §1 rank 9/11) and adopting
them in the already-threaded packaging walkers.

**Code (2 commits):**
1. `12ea8d28` — built `MarkNode` (mark_ptr +1.b32.s1, mark_class +1.b32.s0),
   `AdjustNode` (adjust_ptr +1.b32.s1), `InsNode` (ins_ptr +4.b32.s0,
   split_top_ptr +4.b32.s1), and `UnsetNode` (box-layout width/depth/height +
   `HasListPtr`; deliberately **not** `HasDimensions` — its +4 slot is
   `glue_shrink`, not `shift_amount`, a naming landmine avoided). Promoted
   `AnyNode::{Mark,Adjust,Ins,Unset}` from raw `MemIdx` to `NodePtr<K>`, so
   `classify()` now confines each family's `from_idx_unchecked` to its one
   audited arm. Adopted the typed accessors in `flush_node_list`,
   `copy_node_list`, `hpack_in`, and `vpackage_in`. **Whatsit is now the only
   raw `AnyNode` variant.** 4 `node::` roundtrip tests.
2. (this commit) — built `SaNode` (eTeX sparse-array record: indexed
   `child(mem, i)` at `mem[p+i/2+1].b32.s{0|1}` + `ref_count` at `b16.s0`) and
   fully typed the already-threaded `do_marks` with it, replacing all ~17 raw
   `mem.b32s{0,1}(q+N)` / `b16s0(q)` sparse-array accesses (the token-list
   refcount reads on the *pointed-to* lists stay raw — those aren't a typed
   family). The audited handle is built once at the `do_marks` entry by
   context (no `NODE_type` to classify). 1 `node::` test.

**Disasm gate:** skipped per the session-150 precedent — every new accessor is
an already-ICF-proven shape (`MemPtr`/`Scaled` at +1/+2/+3/+4.b32.s0/s1,
`b16.s0`), so no new shape was introduced.

**Tests:** `cargo check` + `fmt --check`; `node::` **47/47**; full dashboard
**29/2/7/4** (after each slice); rebuilt `target/release/tectonic`; corpus
`plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0
missing-golden**, only the known FP divergences `1905.00073v1` and
`1905.00148v1` (no regression). Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` **125 → 131** (+6: the 4 promoted-family classify arms + 1
`SaNode` test + 1 `do_marks` audited handle — all single-site boundaries; node
74→79, packaging 0→1, math 36, linebreak 12, mem 3); **packaging method-form
`+N` accesses 89 → 52 (−37)**.

**Next:** the only untyped `AnyNode` family left is **Whatsit/native** (`01c`
§1 rank 9) — variable-size, trailing inline UTF-16/glyph data, a raw native
pointer, FFI-measured: the documented "hardest, last" family and a dedicated
slice, not a mechanical batch. Otherwise the remaining levers are all
bucket-C/D threading: the print subsystem (`print_*` → state params, which
unblocks the `shipout`/`output` show-node walkers), the display-math trio
`init_math`/`app_display`/`after_math`, or the `pagebuilder`/`hyphenate`/
`line_break` tails.

---

## Idiom session 178 — 2026-06-13 — Execution: thread/type `mlist_to_hlist` (both passes)

**Type:** execution (workstream 01 / fused `01b`+`01c`). Took the documented
next bridge host — the central math driver `mlist_to_hlist` — and threaded/
typed its tractable islands across both passes. Larger-than-usual session per
user request (4 code commits), batched validation, single doc update. First
committed the inherited sessions 174–177 WIP (`1dbd2808`) as a clean baseline.

**Recon finding (why this shape):** there is no large block of easy bucket-A/B
work left. The remaining raw-accessor mass is in big bucket-C/D hosts
(`main_control` 744 raw `mem_*` calls, `math` 343, `shipout` 309). The
`output.rs` show-node walkers the recon ranks as "B" are actually bucket-C:
`print_char`/`print_esc`/… are `with_state`-bearing, so a walker holding a mem
borrow across a print nest-panics. So "bigger session" = grind *many* islands
through one host, not find easy whole-function threads.

**Code (4 commits, all in `src/math.rs`):**
1. `dfa8a34e` pass-1 reswitch CHOICE_NODE + GLUE_NODE → typed `with_state`
   islands. CHOICE via `classify_choice` + `ChoiceNode` branch accessors +
   threaded `flush_node_list` (new `mlist_choice_node` helper); GLUE via
   `GlueNode` subtype/glue_ptr + threaded `delete_glue_ref`/`flush_node_list`.
2. `9e4b3546` pass-1 action dispatch: new `mlist_track_box` (hpacked box
   height/depth → max_h/max_d + typed `free_node`, one borrow) and
   `mlist_advance` (noad step + style reset; r_type + link read in one borrow);
   applied to ProcessNucleus/CheckDimensions/DoneWithNoad, deduplicating the
   repeated tail.
3. `ba798025` pass-2 islands: BIN_NOAD fixup + list-reset prelude; the
   move-and-detach arm; new-hlist splice + tail walk; inter-element glue
   spacing (threaded `new_glue`); penalty insertion (threaded `new_penalty`);
   final link-read + threaded `free_node`. Removed the dead `new_glue_global`
   import.
4. `c848a53b` ProcessNucleus nucleus store: fold `set q+1 = p` + the
   script-presence check (q+3/q+2 != EMPTY) into one borrow guarding the
   bucket-D `make_scripts_global`.

Bucket-D calls (`make_*_global`, `hpack`, native FFI, `mlist_to_hlist`
recursion) stay outside the borrows by design.

**Tests:** `cargo check`+`fmt --check`; `node::` 42/42; full dashboard
**29/2/7/4** (run after the pass-2 batch); rebuilt `target/release/tectonic`;
corpus `plan/scripts/corpus.sh check all`: **637 passed / 2 diverged / 0 missing-golden**, only the known FP
divergences `1905.00073v1` and `1905.00148v1` (no regression). Metrics:
warnings **41**; `unsafe fn` **568**; `from_idx_unchecked` total **125**
(node 74, math 36, linebreak 12, mem 3, packaging 0; +1 in math from the
GLUE-arm classified-boundary handle); **`math.rs` raw `mem_*` bridge calls
343 → 276 (−67)** — the headline reduction this session.

**Next:** `mlist_to_hlist`'s remaining raw islands are bucket-D-heavy (the
ProcessNucleus MATH_CHAR/SUB_MLIST head: `fetch`, `new_native_character`,
`measure_native_glyph` FFI, `hpack`, recursion). Continue with another bridge
host. `make_op_global`/`make_math_accent_global` are already largely threaded;
the biggest unblocked levers now are threading the **print subsystem**
(`print_*` → state params), which would unblock the whole `shipout`/`output`
show-node-walker family, or the `pagebuilder`/`hyphenate`/`line_break` tails.

---

## Idiom session 177 — 2026-06-13 — Execution: thread/type `make_math_accent_global` islands

**Type:** execution (workstream 01 / fused `01b`+`01c`). Followed session
176 by taking another real bridge host, not pointer-only cleanup.

**Code:** split `make_math_accent_global` into typed helper islands for the
initial accent noad/font snapshot, scripted-base reclean, native glyph setup
around `measure_native_glyph`, box dimension/shift updates, and top/bottom
accent stack assembly. Kept `clean_box`, `vpackage`, OpenType variant/assembly
callbacks, and native glyph measurement outside live `with_state` borrows.
Top-accent kern insertion now uses the typed `KernNode` allocator; repeated
noad conversions in this slice go through one local `assume_noad` boundary.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4**; rebuilt `target/release/tectonic`; final
`plan/scripts/corpus.sh check all`: **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`;
`git diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **124** (node.rs 74, math 35, linebreak 12, mem 3;
packaging 0).

**Next:** continue with another bridge host: `mlist_to_hlist` itself,
`make_op_global`, shipout/output walkers, or the `hyphenate`/`line_break`
tail.

---

## Idiom session 176 — 2026-06-13 — Execution: thread `vsplit` as a coherent packaging slice

**Type:** execution (workstream 01 / fused `01b`+`01c`). Continued the
session-175 packaging WIP by taking the deferred `vsplit` bridge island,
not a pointer-only cleanup.

**Code:**
1. Split `vsplit` into an explicit-state `start_vsplit`/`vsplit_resume`
   core over `MemState`/`EqtbState`/`ScannerState`/`ShipoutState`, while
   keeping the legacy `pub unsafe fn vsplit` wrapper. The core now threads
   through `find_sa_element`, `delete_sa_ref`, `flush_node_list`,
   `do_marks`, `prune_page_top`, `vpackage_in`, `free_node`, and
   `delete_token_ref` instead of chaining the old `_global` bridge calls.
2. Moved the migrated state boundaries into the slice: `scanner.disc_ptr`,
   `scanner.sa_root`, `scanner.cur_mark`, and box-register `eqtb` writes are
   direct. The source box is classified as a typed `NodePtr<VListNode>`.
   `cur_val`/`cur_ptr` stay as raw hot globals, and Mark-node fields stay raw
   because that family is intentionally not typed yet.
3. Reused the resumable `vert_break` core rather than calling the legacy
   wrapper under a borrow. Infinite-shrinkage still pauses through the
   `vsplit` wrapper, prints the original diagnostic after dropping the state
   borrow, repairs the glue spec through the threaded typed helper, and then
   resumes the same `VertBreakCursor`.
4. Plumbed `vpackage_in` outcomes through `VsplitOutcome` so vpack
   diagnostics are printed only after the state borrow drops. Both potential
   packages are preserved and printed in TeX order.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); rebuilt `target/release/tectonic`; ran
`crates/engine_xetex/plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**, only the known `1905.00073v1` and
`1905.00148v1` (script exits nonzero because it reports those known diffs);
`git diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** packaging's `vert_break`/`vsplit` bridge island is now threaded.
Continue with another real bridge host (`mlist_to_hlist`, shipout/output
walkers, `make_op_global`/`make_math_accent_global`, or the
`hyphenate`/`line_break` tail). Do not spend the next slice on pointer-only
cleanup.

---

## Idiom session 175 — 2026-06-13 — Execution: thread and type `vert_break`

**Type:** execution (workstream 01 / fused `01b`+`01c`). Targeted the
remaining packaging global-accessor island instead of another pointer cleanup.

**Code:**
1. Split `vert_break` into a resumable explicit-state core over
   `MemState`/`ScannerState` and kept the legacy `pub unsafe fn vert_break`
   as the bridge wrapper. The core no longer calls `mem_b16s1`,
   `mem_b32s1`, `active_width`, or `best_height_plus_depth` globals; it
   reads/writes `scanner.active_width` and `scanner.best_height_plus_depth`
   directly.
2. Adopted typed/classified access in the new core: list-node dispatch goes
   through `MemState::classify`; HList/VList/Rule dimensions use typed
   accessors; Glue/Kern/Penalty arms use `GlueNode`, `GlueSpecNode`,
   `KernNode`, and `PenaltyNode` methods. Whatsit PIC/PDF and untyped
   Ins/Mark behavior remains raw as planned.
3. Preserved the mid-loop infinite-shrinkage error ordering by making the
   core pause with a `VertBreakStep::InfiniteShrinkage`. The wrapper drops
   the state borrow, emits the original diagnostic/error, then reborrows to
   copy the glue spec with threaded `new_spec`, set the copied
   `shrink_order` through the typed glue-spec setter, delete the old ref via
   threaded `delete_glue_ref`, and resume the same TeX loop state.

**Not done:** `vsplit` was inspected but left as the next slice. It still
crosses several bridge boundaries (`find_sa_element_global`, mark state,
`prune_page_top_global`, `vpackage`, box-register writes); threading it
coherently should pull those boundaries together rather than just retyping
raw fields in place.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **42/42**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); rebuilt `target/release/tectonic` and ran
`plan/scripts/corpus.sh check all`: **637 passed, 2 diverged, 0
missing-golden**, only the known `1905.00073v1` and `1905.00148v1`; `git
diff --check`. Metrics: warnings **41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** thread `vsplit` as a coherent slice, probably including the mark
collection loop, sparse-array calls, `prune_page_top`, and `vpackage` outcome
plumbing so the function can stay under explicit state without nested
bridges. Keep Unset/Ins/Mark/Adjust/Whatsit fields raw unless their typed
surfaces are deliberately built.

---

## Idiom session 174 — 2026-06-13 — Execution: prettify typed packaging arm bodies

**Type:** execution (workstream 01 / `01c` §3 follow-through). Continued
the session-173 packaging work: arms already received typed `AnyNode`
handles, but several bodies still reached back through raw `p + N` /
`r + N` offsets for built families.

**Code:**
1. Added a small `copy_box_shell` helper for `just_copy`/`copy_node_list`:
   it copies the full box shell, classifies the copied header as an
   `AnyBox`, and sets `list_ptr` through the typed setter. HList/VList
   copy paths now read child lists through `list_ptr()`; `Unset` stays raw
   because it is not typed yet.
2. Retyped glue/ligature/disc copy bodies: source fields now come from
   `GlueNode::{glue_ptr,leader_ptr}`, `LigatureNode::lig_ptr`, and
   `DiscNode::{pre_break,post_break}`; copied destination nodes copy their
   header first, classify, then write destination pointer fields via typed
   setters. This removed the old `mem_b32 p + 1` style bodies while keeping
   allocation sizes and copy order equivalent.
3. Retyped more already-classified consumers: `flush_node_list` uses typed
   choice branch pointers; `prune_page_top` uses typed HList/VList/Rule
   heights via a factored split-top-skip helper; `hpack`/`vpackage` measure
   leader boxes/rules through typed dimensions when classification succeeds
   (fallback remains the old raw read for untyped/unexpected leader records);
   `hpack`'s ligature temporary char-node now uses ligature `font`,
   `character`, and `link` accessors.
4. Removed the dummy `r = 0` initialization in `copy_node_list`; the
   compiler now verifies every copy path assigns `r`.

**Left raw deliberately:** `Unset`, `Ins`, `Mark`, `Adjust`, and `Whatsit`
fields in these walkers still use raw offsets because those families are not
typed yet. `vert_break`/`vsplit` still read through global `mem_*` accessors;
threading them is a separate fused slice, not an arm-body cleanup.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; filtered `node`/`units` tests **46/46**; dashboard
**29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`);
`git diff --check`. Metrics: warnings **42 → 41**; `unsafe fn` **568**;
`from_idx_unchecked` total **123** unchanged (node.rs 74, math 34,
linebreak 12, mem 3; packaging 0).

**Next:** the remaining packaging raw offsets are either untyped-family
fields or unthreaded/global islands. Continue with a real fused threading
slice (`vert_break`/`vsplit`, `mlist_to_hlist`, shipout/output walkers), or
build typed surfaces for the low-rank families still represented as raw
`MemIdx` (`Unset`, `Ins`, `Mark`, `Adjust`, selected Whatsit/native records).

---

## Idiom session 173 — 2026-06-13 — Execution: adopt `classify` across the packaging list-walkers (large, low-docs)

**Type:** execution (workstream 01 / `01c` §3 — the session-172 "Next"
item: clean `classify` adoptions). 2 commits.

**Code:**
1. `flush_node_list` / `copy_node_list` / `just_copy` now dispatch on
   `MemState::classify(p) -> AnyNode` instead of a raw `match
   mem.b16s1(p)` over `*_FNL`/`*_CNL` constants. This **removes all 3 of
   packaging's `from_idx_unchecked`** (the `flush_node_list` GLUE arm now
   binds the typed `NodePtr<GlueNode>` straight from `AnyNode::Glue`, and
   its three noad arms — ORD..ACCENT / LEFT|RIGHT / FRACTION — collapse
   into one `AnyNode::Noad` arm sub-dispatching on `noad_type`).
   Per-walker exhaustiveness preserved: `flush_node_list`'s `AnyNode`
   match is total (no catch-all); `copy_node_list` keeps `_ =>
   confusion("copying")` for the valid-but-uncopied variants
   (noad/style/choice) and `just_copy` keeps `_ => None` (silent skip),
   both reached via `classify().map_err(...)?` / `classify().ok()`. Dead
   `*_FNL`/`*_CNL` node-type consts + now-unused imports removed.
2. `prune_page_top` adopts the same surface (`AnyNode::HList|VList|Rule`,
   `Whatsit|Mark|Ins`, `Glue|Kern|Penalty`, `_ => confusion("pruning")`);
   dead `*_PPT` node-type consts + `HLIST_NODE`/`RULE_NODE` imports
   removed.

**Behaviour-preserving:** every typed arm lowers to the same field
access the raw match performed; `classify`'s `Err` (invalid type code)
maps to the exact catch-all each walker already had.

**Not done (recorded):** `ignorable_protrusion_node_in` (linebreak) and
the math noad helpers are left raw — linebreak's `*_node()` /
`from_idx_unchecked` are the **intentional `*_in`/confinement layer**
(`01c` §6, session-172 note), not classify targets; the math sites are
justified post-dispatch boundaries in helpers that receive an
already-typed node from a caller (`mlist_to_hlist`) that is itself still
unthreaded. The threaded-function `classify`-adoption vein is now
**exhausted** — every remaining `match mem.b16s1` dispatch loop
(`shipout`/`output`/`math.rs` `mlist_to_hlist`) reads through the
`mem_b16s1` **global** accessor, i.e. is unthreaded, so adopting
`classify` there requires threading the host first (bucket C/D).

**Tests:** `cargo check`; `cargo fmt --check`; `node::` **42/42**;
dashboard **29/2/7/4**; release corpus exit 0, only the known FP
divergences `1905.00073v1` / `1905.00148v1` (**no regression**).
Metrics: warnings **43 → 42**; `unsafe fn` **568**; `from_idx_unchecked`
total **126 → 123** (packaging **3 → 0**; node.rs 74, math 34,
linebreak 12, mem 3).

**Next:** the mechanical `classify`/consumer vein is exhausted; the
remaining levers are all **threading** large bridge/global functions so
their dispatch loops can then adopt `classify` in the same fused slice
(`01b` §3a): `mlist_to_hlist` (math), the `shipout`/`output`
node-walkers, `vert_break`/`vsplit`, `make_op_global`/
`make_math_accent_global`, the `hyphenate`/`line_break` tail. No more
pointer-only or raw-dispatch-only passes.

---

## Idiom session 172 — 2026-06-13 — Execution: unified `AnyNode` + `classify`, adopted in hpack/vpackage

**Type:** execution (workstream 01 / `01c` §3 — the single classification
boundary). Prompted by user design question: the big `match NodeType` loops
each did a per-arm `from_idx_unchecked`; can the typed handle be found
automatically?

**Answer + code (2 commits):**
1. Built the unified surface the plan specified but never landed (`01c` §3):
   `AnyNode` (typed `NodePtr<K>` per built family; raw `MemIdx` for the
   not-yet-typed native/whatsit/unset/ins/mark/adjust) + `MemState::classify(p)
   -> Result<AnyNode, InvalidNodeType>`. It confines every built family's
   `from_idx_unchecked` to one audited site; a consumer matches `AnyNode` and
   writes no `unsafe`, with the static type inferred per arm. Noads collapse to
   one `Noad` variant (refine via `noad_type`). Context-disambiguated codes are
   classified by node-list meaning (2→Rule not Delta, 14→Style not Edge).
   **Disasm gate (`01c` §4): a comprehensive typed classify-dispatch compiles
   to a jump table → direct field load, no enum materialized — tighter codegen
   than the equivalent raw multi-arm `NodeType` match (123 vs 217 bytes).** Unit
   test `classify_dispatches_to_typed_handles`.
2. Adopted it in the `hpack`/`vpackage` main loops: `match
   mem.classify(node_idx)` replaces `match NodeType::from_raw(...)` + 11 per-arm
   `from_idx_unchecked`. The Ins|Mark|Adjust arm binds the matched value (`arm
   @ (..)`) to recover the Adjust discriminant. `packaging.rs`
   `from_idx_unchecked` 14→3 (the rest are `flush_node_list`, which dispatches
   on raw `*_FNL` constants — a separate, larger adoption).

**Tests:** `cargo check`; `node::` **42/42**; dashboard **29/2/7/4**; release
corpus exit 0, only known FP divergences `1905.00073v1`/`1905.00148v1` (**no
regression**). Metrics: warnings **43**; `unsafe fn` **568**; `from_idx_unchecked`
total **126** (node.rs **74** = the audited confinement layer + classify +
per-family classifiers + allocs + tests; packaging **3**, math **34**,
linebreak **12** = confined `*_in` helpers, shipout **0**).

**Next clean `classify` adoptions:** `flush_node_list`/`copy_node_list`/
`just_copy` (restructure the raw-`*_FNL`-constant match to `AnyNode`, with the
noad arm sub-dispatching on `noad_type` and the box arm grouping
HList|VList|Unset), then `mlist_to_hlist` (math, 34 sites). linebreak's
`from_idx_unchecked` are the intentional `*_in` confinement helpers, **not**
classify targets.

---

## Idiom session 171 — 2026-06-13 — Execution: land hpack/vpackage thread, drop non-fix unsafe helpers, type more consumers

**Type:** execution (workstream 01 / `01b` §3a). Large low-docs session.

**Code (3 commits):**
1. Validated + cleaned + committed the inherited uncommitted WIP that
   threads `hpack`→`hpack_in` / `vpackage`→`vpackage_in` over explicit
   state, splitting overfull/underfull diagnostics into borrow-dropping
   wrappers (`print_{h,v}pack_diag`/`_common_ending`, `PackDiag`) and
   adopting `classify_box` + `VListNode::alloc`. Removed dead
   `total_stretch`/`total_shrink` setters and the node-type constants the
   threaded packers stopped referencing.
2. **Design fix (user-flagged):** deleted the never-wired
   `AnyBox::from_node_type` / `RuleNode::from_node_type` helpers the WIP
   added. Relocating `from_idx_unchecked` into a per-call helper does NOT
   eliminate the unsafe tag→type assertion; `classify_box`/`classify_*`
   are the single audited boundary that turns a runtime `NODE_type` into a
   typed handle, after which each `match` arm carries the static type with
   zero further unsafe.
3. Typed more box/glue/noad/choice consumers in already-threaded fns:
   packaging `flush_node_list` GLUE arm (close the glue family), math
   `apply_math_limit_switch`, `stack_glue_into_box` (glue spec),
   `build_ext_delimiter_box` (vlist dims), `first_script_glyph_step`
   (choice mlists), `rebox` (classify-once box), shipout
   `finish_{h,v}list_rule` (classify output box).

**Tests:** `cargo check`; `node::` **41/41**; dashboard **29/2/7/4**.
Rebuilt `target/release/tectonic`; `plan/scripts/corpus.sh check all`
exit 0 with only the known FP divergences `1905.00073v1` /
`1905.00148v1` (**no regression**). Metrics: warnings **43**,
`with_state` **1156**, `unsafe fn` **568**, `#[no_mangle]` **11**, bare
`[pqr]: i32` **347** (up from new threaded packer helper params; retires
as callers thread).

**Finding for next session:** the cheap consumer-typing vein in
already-threaded packaging/math/shipout fns is now genuinely thin (this
session + prior ones largely exhausted it). The remaining big levers are
**threading large bridge-based functions**: `vert_break`/`vsplit`
(packaging; the mid-loop "infinite glue shrinkage" print+continue path
blocks a single borrow — bucket D), `make_op_global`/`make_math_accent_global`
(math), `hyphenate`/`line_break` tail (linebreak, 65/47 bridge sites),
and the still-unthreaded shipout/pagebuilder/output/main_control/scan
bodies. Each is a careful bucket-C/D slice, not a mechanical batch.

---

## Idiom session 170 - 2026-06-12 - Execution: typed shipout records + threaded `post_line_break`

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** added typed `EdgeNode` and untagged shipout `MovementNode` records in
`node.rs`, then adopted them directly in `shipout.rs` `new_edge`,
`prune_movements`, and `movement`. Split `post_line_break` around the remaining
global `hpack` boundary: the prefix and suffix now thread explicit state and use
typed `DiscNode`, `GlueNode`, `GlueSpecNode`, `KernNode`, `MathNode`,
`PassiveNode`, and `AnyBox` accessors. Removed dead one-line wrappers/accessors
for margin-kern insertion, LR-stack update, `set_just_box`, linebreak
width/indent getters, and `best_line`. The split initially exposed stale
`just_box` state in display math; fixed by restoring `scanner.just_box` before
the suffix calls paths that still read the global bridge.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p
tectonic_engine_xetex`; `cargo test -p tectonic_engine_xetex node::` **41/41**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`); `git diff --check`. Rebuilt `target/release/tectonic`;
corpus returned **637 passed, 2 diverged, 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Metrics: warnings **43**, `with_state`
**1155**, `unsafe fn` **568**, direct `#[no_mangle]` grep **11**, current bare
`[pqr]: i32` grep **341**.

**Next:** `hpack` is the main remaining boundary in this area; next large slice
should thread and type `hpack`/packaging or another closed dependency. Avoid
pointer-only helper passes.

---

## Idiom session 169 — 2026-06-12 — Execution: inline delta fields + post-line-break entry

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Low-docs mechanical slice.

**Code:** removed the index-based `delta_node_width_in` /
`set_delta_node_width_in` wrappers and inlined direct `DeltaNode`
`dwidth`/`dstretch*`/`dshrink` getters and setters throughout the already
threaded `try_break` and `deactivate_node` paths. Threaded the first
`post_line_break` island, `reverse_best_breaks`, over `MemState` +
`LinebreakState` and typed it through `ActiveNode`/`PassiveNode`; deleted the
now-dead `globals::best_bet` one-line bridge.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; corpus returned **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Metrics:
warnings **43**, `with_state` **1162**, `unsafe fn` **571**, direct
`#[no_mangle]` grep **11**, current bare `[pqr]: i32` grep **343**.

**Next:** continue `post_line_break` only in closed slices that can thread
state and type nodes together; `hpack` remains a boundary until it is threaded.

---

## Idiom session 168 — 2026-06-12 — Execution: inline typed linebreak consumers

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued the existing uncommitted linebreak WIP. Per user direction,
did not grow more one-line field-helper wrappers; kept context-boundary
constructors and inlined typed calls inside threaded bodies. Typed more
already-threaded linebreak/protrusion islands: `total_pw` active/passive/disc
fields, active/delta/passive links in setup/cleanup/best-break/`try_break`
deactivation, margin-kern insertion links via `MarginKernNode`, and
`update_lr_stack` math subtype reads via `MathNode`. Native/whatsit and legacy
`line_break`/`post_line_break` raw islands remain for later closed thread+type
slices.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; corpus returned **637 passed, 2 diverged, 0
missing-golden**, only known `1905.00073v1` and `1905.00148v1`. Metrics:
warnings **43**, `with_state` **1162**, `unsafe fn` **571**, direct
`#[no_mangle]` grep **11**, current bare `[pqr]: i32` grep **277**.

**Next:** thread `post_line_break` only as a closed slice with its
allocation/global dependencies, or continue already-threaded linebreak helper
raw islands. Do not add one-line field-helper wrappers that will be removed
later.

---

## Idiom session 167 — 2026-06-12 — Execution: typed `linebreak` main-loop consumers

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued from the existing uncommitted linebreak WIP. Added typed
context-boundary helpers for already-classified linebreak/main-list families
(`GlueNode`/`GlueSpecNode`, `KernNode`, `LigatureNode`, `DiscNode`,
`MathNode`, `PenaltyNode`, `RuleNode`, `AnyBox`) and used them immediately in
threaded `try_break` and protrusion helpers. `try_break` now computes
discretionary widths, trailing discard widths, glue-spec stretch/shrink, math
width, kern width/subtype, ligature font/char, and passive/active/delta
allocation setup through typed handles instead of raw `mem[p+n]` fields. Added
typed active/delta list-link accessors in `node.rs`. Kept native/whatsit raw;
that family is not typed yet.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; `CANDIDATE_BIN=... plan/scripts/corpus.sh check all`
returned **637 passed, 2 diverged, 0 missing-golden**, only known
`1905.00073v1` and `1905.00148v1`. Final post-format local rename only changed
helper parameter names; final `cargo check` still green. Metrics: warnings
**43**, `with_state` **1162**, `unsafe fn` **571**, direct `#[no_mangle]`
grep **11**, bare `[pqr]: i32` params **349**.

**Next:** finish the remaining `line_break` main-loop raw islands or move into
`post_line_break`; native/whatsit typing remains a separate family-sized slice.

---

## Idiom session 166 — 2026-06-11 — Execution: thread more `line_break` setup/teardown

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed-node
closure). Larger mechanical slice; docs intentionally minimal.

**Code:** continued upward from `try_break` into `line_break` without adding
pointer-only intermediate helpers. Added threaded, typed helpers for pass setup
(`start_linebreak_pass`), active/passive cleanup, best-break selection, and the
last-line-fit glue adjustment. These use direct `MemState`/`ScannerState`/
`LinebreakState`/`EqtbState`/`ErrorState` fields plus `NodePtr<ActiveNode>`,
`GlueNode`, and `GlueSpecNode`; allocation failures still defer through
`NodeRecycleAbort` so aborts run after the borrow drops. Also typed the
already-threaded `finite_shrink` glue-spec shrink-order write.

**Cleanup:** removed the dead active-node/delta/passive linebreak bridge
wrappers and the now-dead `globals.rs` linebreak accessors exposed by this
slice (`set_first_p`, best setters, passive accessors, active-node-size getter,
`do_last_line_fit`, etc.). Warnings remained at **43**.

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic` after the final tree and ran
`CANDIDATE_BIN=... plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`.
Metrics: `with_state` **1162**, `unsafe fn` **571**, direct `#[no_mangle]`
grep **11**, bare `[pqr]: i32` params **344**.

**Next:** continue through the remaining `line_break` main-loop islands and
then `post_line_break`, or switch to threading `hpack`/`vpackage`.

---

## Idiom session 165 — 2026-06-11 — Execution: thread `try_break` directly

**Type:** execution (workstream 01 / `01b` Phase-C threading + typed consumer
closure). Large mechanical session; docs intentionally minimal.

**Code:** threaded `linebreak.rs::try_break` to explicit
`&mut MemState`, `&mut ScannerState`, `&mut LinebreakState`,
`&EqtbState`, `&mut FontState`, `&ConfigState`, and `&mut ErrorState`.
Inside the threaded core, active/delta/passive linebreak records now use the
existing typed handles directly (`active(...)`, `delta_node_width_in`,
`passive_node(...)`) and the linebreak/scanner arrays are accessed as state
fields. Cold `get_node` exhaustion, `confusion`, and the `total_pw` pdf-error
path are deferred through a narrow `TryBreakAbort` so aborts run after the
thread-local borrow drops. Legacy `line_break` call sites now enter through
`try_break_global`.

**Cleanup:** deleted the now-dead linebreak bridge wrappers
(`delta_node_width`, passive setters, active break/fitness getters,
`total_pw_global`) and removed dead `globals.rs` bridge accessors for
`try_break`-private fields (`cur_active_width`, `break_width`, best-place/best
line arrays, minimal/minimum/easy/fill/best-pl reads and dead setters).

**Tests:** `cargo check -p tectonic_engine_xetex`; `cargo test -p
tectonic_engine_xetex node::` **41/41**; dashboard **29/2/7/4**
(`tex-outputs`, `trip`, `engine_rust`, `latex_goldens`). Rebuilt
`target/release/tectonic`; `plan/scripts/corpus.sh check all` produced the
expected **637 passed, 2 diverged, 0 missing-golden**, with only known
divergences `1905.00073v1` and `1905.00148v1`. Metrics: warnings 43,
`with_state` 1177, `unsafe fn` 571, direct `#[no_mangle]` grep 11.

**Next:** continue upward into `line_break` setup/teardown so the remaining
active-node setter wrappers can disappear at a real threaded boundary, or move
to `hpack`/`vpackage`.

---

## Idiom session 164 — 2026-06-11 — Execution: typed math/linebreak consumers (larger mechanical batch)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Large
mechanical session per user request; docs intentionally minimal. 2 commits.

**Commits:**
1. `28fcf31d` `engine_xetex: type more node consumers`
   - Committed the inherited `linebreak.rs` active-list typed accessor wrapper
     (`ActiveNode` around the old active helper layer).
   - Typed several `math.rs` consumer islands without entering the native
     whatsit family: `make_op_global`'s LIMITS/math-char mem clusters,
     `compute_ot_math_accent_pos_step`, and box-side native-glyph helpers.
2. `1d7346af` `engine_xetex: type linebreak delta consumers`
   - Added narrow `DeltaNode`/`PassiveNode` bridge helpers and used them in
     `try_break` / `deactivate_node` for delta widths and passive break fields.

**Tests:** `cargo check -p tectonic_engine_xetex`; `node::` **41/41**;
dashboard **29/2/7/4** (`tex-outputs`, `trip`, `engine_rust`,
`latex_goldens`). Rebuilt `target/release/tectonic` and ran
`CANDIDATE_BIN=... plan/scripts/corpus.sh check all`: **637 passed, 2
diverged, 0 missing-golden**. The divergences were only the known
`1905.00073v1` and `1905.00148v1`.

**Notes:** two explorers were useful for target selection: both pointed at
`make_op_global`/linebreak internals and away from `make_math_accent_global` as
a full-threading target. User correctly called out that helper-per-field
wrappers like `active_node_total_demerits_in` are transitional ceremony now that
typed pointers exist; future linebreak work should collapse toward direct typed
handles inside threaded cores instead of growing this layer. Metrics after the
session: warnings 43, `with_state` 1202, `unsafe fn` 571, direct
`#[no_mangle]` grep 11.

**Next:** thread `try_break`/linebreak internals properly and delete the
transitional helper wrappers as raw pointers enter typed cores; or thread
`hpack`/`vpackage`. Avoid adding more legacy bridge wrappers except at real
raw-to-typed boundaries.

---


## Idiom session 163 — 2026-06-10 — Execution: subscr/supscr rename + bulk noad & glue-spec consumer typing (large, low-docs)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Large mechanical
session per user request (less docs, more code). 5 commits.

**Commits:**
1. `e8c364db` **subscr/supscr rename slice** (the session-162 landmine fix).
   Offset-preserving swap of the inverted `NodePtr<Noad>` accessor names: now
   `supscr_*` = +2, `subscr_*` = +3 (matching TeX and this build). Method
   *definitions* (node.rs) and *every call site* (math.rs, mem.rs) were swapped
   together via a `subscr_`↔`supscr_` identifier swap (perl on math.rs/mem.rs,
   hand-rewrite of node.rs methods + addr getters), so each site still hits the
   same word — behaviour-preserving, compiler-checked. Removed the landmine
   comment. This unblocks typing noad consumers by semantic name.
2. `818b95a2` typed noad consumers in **math.rs**: `make_over`,
   `make_under_finish`, `make_vcenter`, `make_fraction_finish` (nucleus
   field/type writes via `NodePtr<Noad>`); `append_script_box`,
   `finish_subsup_stack_global`, `finish_superscript_branch_global`,
   `make_scripts_global`, `make_left_right_global` (nucleus/supscr/subscr reads
   + new_hlist/noad_type). Each function's noad family fully closed.
3. `20c6283b` typed noad consumers in **packaging.rs** `flush_node_list`:
   standard-noad arm (Ord..Accent) + fraction-noad arm (numerator=`supscr_field`
   +2, denominator=`subscr_field` +3). Box arm left raw (also matches
   `UNSET_NODE`, which `classify_box` rejects).
4. `1ff467c7` typed the glue-spec **`ref_count` trap** (the canonical
   "ref_count is a count, NOT a link") at 4 **packaging.rs** sites: `just_copy`
   (GLUE), `copy_node_list` (INS + GLUE), `flush_node_list` (GLUE), via
   `NodePtr<GlueSpecNode>::from_glue_ptr` + `ref_count`/`set_ref_count`.

**Method:** match **offset → node.rs accessor** mechanically; close one family
per function (per-family closure rule). Behaviour-preserving — every typed
accessor lowers to the identical raw field access. **Deliberately left raw**
(recorded, not regressions): the bridge-mixed math hosts `make_op_global`,
`make_math_accent_global`, and the giant `mlist_to_hlist` (they access noad/box
fields via BOTH `mem_b32s1` bridges and `st.mem`, so typing one half breaks the
closure rule — thread them whole first); the box arms of
`flush_node_list`/`copy_node_list` (UNSET_NODE shares the layout but
`classify_box` rejects it); linebreak.rs `*_in` accessor helpers (already a
clean named confinement layer — re-routing through `NodePtr` is redundant).

**Tests:** `node::` 38/38; dashboard **29/2/7/4** (every commit). Corpus
(`plan/scripts/corpus.sh check all`): **637 passed, 2 diverged, 0
missing-golden** — the 2 are the pre-existing known arXiv divergences
(`1905.00073v1`, `1905.00148v1`, tier2 FP/native-text), matches baseline, NOT
regressions. Metrics: `with_state` 1197, `unsafe fn` 571 (unchanged — pure
consumer typing, no new threading), warnings 43.

**On subagents (user asked):** the remaining work is concentrated in single files
(math.rs, packaging.rs) and is parity-sensitive, so parallel subagents would
just conflict on the same file and risk silent offset bugs; serial edits with the
dashboard as the gate were the safer high-throughput path. Cross-file
parallelism isn't available yet because the other high-traffic files aren't
`mem`-threaded (packaging has 0 `mem:&MemState` fns outside the few node-walkers;
linebreak/shipout are mostly bridge/global). A subagent ran the background corpus
check while editing continued.

**Next:** the cheap box/noad/glue *consumer* vein in already-threaded functions
is now largely exhausted. Highest-value next steps are **threading** work, not
consumer typing: (a) thread `make_op_global` / `make_math_accent_global`
end-to-end (convert their `mem_b32s1` bridges to `st.mem`/typed) so their
box+noad families can be closed; (b) build a typed family for the rule-11
page/linebreak internals (active/passive/movement/mark/ins) or the native/whatsit
family (rank 9) and adopt consumers in shipout/copy_node_list; (c) thread
`hpack`/`vpackage` (the canonical box-dimension consumers, currently on bridges).

---

## Idiom session 162 — 2026-06-10 — Execution: bulk box-dimension consumer typing in math.rs (large mechanical batch)

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Intentionally
large, low-docs session per user request — typed-consumer adoption only (no new
node families). Two commits.

**Code (10 functions, ~21 raw box accessors retyped; math.rs 145→124
method-style accessors):**
- Commit 1 (`2ff59e2f`): `add_script_space_and_restore_font`,
  `initial_script_shifts`, `finish_subsup_stack`, `finish_subscript_only`,
  `prepare_superscript`, `finish_superscript_branch`, `make_under_finish`,
  `make_vcenter` — raw `mem.b32s1(box+N)` → typed `AnyBox`
  width/depth/height/shift_amount/list_ptr/link via `classify_box`. Dropped the
  now-unused `AnyBox` import (warnings 44→43).
- Commit 2 (`adbd33f9`): `char_box` (the built box `b`), `make_fraction` (vbox
  `v` + numerator `x` / denominator `z`).

**Method:** match **offset → node.rs accessor** mechanically (node.rs is the
test-validated offset authority). Each function's **box family is fully closed**
(per-family closure rule, `01b` §3a); other families in the same body
(noad `q+N`, native-glyph `p`, kern/rule links) left raw — separate slices.
Behaviour-preserving: every typed accessor lowers to the identical raw field
access.

**Skipped (recorded, not regressions):** `make_op_global` is only *partially*
threaded (mixes `set_mem_b32s1` bridge calls and `st.mem.` closures on the same
box `x`) — typing only the closure half would violate the closure rule. Thread
the whole function first. `make_math_accent` box reads (`x+1`/`x+3`) likewise
sit in a large body with other raw box accesses; defer to a whole-function
slice.

**⚠ FINDING — node.rs subscr/supscr names are inverted vs TeX (offset-correct,
NAME-wrong):** `NodePtr<Noad>::subscr_*` reads **offset +2**, `supscr_*` reads
**offset +3**. But TeX (and this build, per `prepare_subscript_global(q + 3)`)
has **supscr = +2, subscr = +3**. The methods are used consistently by offset
(dashboard green), so **no behaviour bug ships** — but the names are a landmine:
a future consumer that calls `subscr_type` expecting the subscript gets the
*superscript* word. This is why this session left all noad `q+2`/`q+3` reads
**raw** rather than adopt the mislabelled accessors. **Recommended next:** a
dedicated, atomic rename slice that swaps the `subscr_*`↔`supscr_*` method names
*and* every call site name together (net offset unchanged at every site →
behaviour-preserving; compiler-checked; gate on the dashboard). Until then,
match noad fields by offset, not by these names.

**Tests:** `node::` 38/38; dashboard **29/2/7/4** (both commits). Corpus
(`plan/scripts/corpus.sh check all`): **637 passed, 2 diverged, 0
missing-golden** — the 2 are the pre-existing known arXiv divergences
(`1905.00073v1`, `1905.00148v1`, tier2 FP/native-text), NOT regressions; matches
baseline. Metrics: `with_state`
1198, `unsafe fn` 571 (unchanged — pure consumer typing, no new threading).

**Next:** (a) the subscr/supscr rename slice (above) to unblock safe noad
typing; then (b) more box-dimension consumers once their host functions are
fully threaded (`make_op_global`, `make_math_accent`); or (c) thread
`make_op_global` end-to-end so its box `x` can be typed.

---

## Idiom session 161 — 2026-06-08 — Execution: MarginKernNode implementation and `attach_hkern_to_new_hlist` refactoring

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). MarginKernNode & Kern attachment.

**Code:** Added `MARGIN_KERN_NODE` (type 40) and `MARGIN_KERN_NODE_SIZE` (3) constants to `src/consts.rs`. Defined `MarginKernNode` marker, its `alloc`/`width`/`set_width` implementations, `HasNodeHeader` implementation, and `MemState::classify_margin_kern` in `src/node.rs`. Added address getters (`nucleus_addr`, `subscr_addr`, `supscr_addr`) to `NodePtr<Noad>`. Refactored `new_margin_kern` in `src/mem.rs` to use `MarginKernNode` and typed accessors. Refactored `attach_hkern_to_new_hlist` and `attach_hkern_to_new_hlist_global` in `src/math.rs` to adopt typed node pointers.

**Tests:** `node::` **38/38** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings went from 43 to **44**.

**Next:** implement safe `maybe_attach_script_kern` core.

---

## Idiom session 160 — 2026-06-08 — Execution: typed math list and radical consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed math list and radical consumers.

**Code:** Added `noad_type` getter to `NodePtr<Noad>`. Added delegation methods `set_width`, `set_depth`, `set_height`, `set_shift_amount`, `list_ptr`, `set_list_ptr`, `link`, and `set_link` to `AnyBox`. Refactored `fin_mlist` in `src/math.rs` to use typed `NodePtr<Noad>` instead of raw macro reads and writes. Refactored `build_choices_finish` in `src/math.rs` to use typed `NodePtr<ChoiceNode>`. Refactored `overbar`, `make_radical_finish`, and `make_radical_prepare_delimiter` in `src/math.rs` to use typed `NodePtr<KernNode>`, `NodePtr<RuleNode>`, `NodePtr<Noad>`, and `AnyBox` accessors.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings went from 42 to **43**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) or remaining raw constructor families.

---

## Idiom session 159 — 2026-06-08 — Execution: typed glue spec constructors and consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed glue spec.

**Code:** refactored `new_spec`, `new_skip_param`, and `delete_glue_ref` in `src/mem.rs` to use typed getters and setters from `NodePtr<GlueSpecNode>` instead of raw macro reads and writes. Also refactored the legacy bridges `math_glue_global` and `math_kern_global` in `src/math.rs` to adopt the typed interfaces for `GlueSpecNode` and `KernNode` respectively, ensuring that the `scaled_mu_value` side-effect calls (which mutably borrow state) occur outside of the state-borrowing closures to avoid `RefCell already borrowed` panics. Cleaned up unused imports `SMALL_NODE_SIZE` from `src/mem.rs` and `get_node` from `src/math.rs`.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings dropped from 44 to **42**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) or remaining raw constructor families.

---


## Idiom session 158 — 2026-06-08 — Execution: typed math noad consumers adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Typed math noad consumers.

**Code:** added generic `link` and `set_link` getters/setters on `NodePtr<K>` where `K: HasNodeHeader` to traverse node lists in a typed manner. Added character and family getters and setters on `NodePtr<Noad>` for the math fields (nucleus, subscript, superscript) and the accent character field. Refactored the core ligature/kern math processor `make_ord_body` and its wrapper `make_ord_global` in `src/math.rs` to adopt the typed `NodePtr<Noad>` interfaces. Also refactored `finish_accent_math_field` to use typed getters and setters. Extended the `noad` unit test in `src/node.rs` to round-trip and verify all the new getters and setters.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed at **44**.

**Next:** continue implementing typed consumers in `math.rs` or `packaging.rs` (e.g. dimension-reading leaves) now that noad field getters/setters are available.

---


## Idiom session 157 — 2026-06-08 — Execution: typed noad constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Constructor typing slice.

**Code:** added unified `Noad` typed surface with multiple type-stamping allocators (for standard noads of size 4, radical/accent variants of size 5, and fraction variants of size 6). Added getters and setters for math fields (nucleus, subscr, supscr), fraction thickness, and accent character. Also added helpers to get the sub-element delimiter addresses. Adopted the surface in `new_noad`. Removed unused `NOAD_SIZE` and `ORD_NOAD` constants from `src/mem.rs`.

**Tests:** `node::` **34/34** unit tests pass. Dashboard green: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed at **42**.

**Next:** either type `new_lig_item` as a non-header lig-stack record, or begin implementing typed consumers of these typed surfaces in `math.rs`/`packaging.rs`.

---


## Idiom session 156 — 2026-06-08 — Execution: typed style/choice constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Constructor
typing slice.

**Code:** added `StyleNode` and `ChoiceNode` typed surfaces with type-stamping
allocators. `ChoiceNode` exposes the four nullable math-list alternatives:
display/text/script/scriptscript. Adopted the surfaces in `new_style` and
`new_choice`. Deliberately skipped `new_lig_item` for now because it is a
lig-stack record with no `NODE_type` header, not a normal list node.

**Tests:** `node::` **32/32**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**.

**Next:** either type `new_lig_item` as a non-header lig-stack record, or start
the noad constructor surface (`new_noad`). Typed consumers in math/packaging
also remain good targets.

---


## Idiom session 155 — 2026-06-08 — Execution: typed glue/math constructor adoption

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). Longer
typed-pointer constructor session, intentionally light docs.

**Code:** added `NodePtr::<GlueNode>::alloc`, preserving this XeTeX build's
`MEDIUM_NODE_SIZE` glue allocation because SyncTeX uses the extra word. Adopted
it in `new_glue` and `new_param_glue`; both now fill `subtype`/`leader_ptr`/
`glue_ptr` through typed setters and bump the target `GlueSpecNode` refcount
through typed accessors. Added the narrow `MathNode` type for `MATH_NODE`
width/subtype and adopted it in `new_math`. Noads remain untouched.

**Tests:** `node::` **28/28**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**.

**Next:** remaining raw constructor families: `new_lig_item` if it deserves a
narrow typed surface, then noad/style/choice. Also good: typed consumers in
packaging/math/shipout now that the common list-node constructors are typed.

---


## Idiom session 154 — 2026-06-08 — Execution: typed rule/ligature/disc allocation + `append_to_vlist` box typing

**Type:** execution (workstream 01 / `01b` §3a + `01c` Stage 3). One larger
typed-pointer session.

**Code:** added `RuleDimen` (`Running` vs `Set(Scaled)`) and `RuleNode`, then
typed `new_rule` and `fraction_rule`. Added `LigatureNode` and typed
`new_ligature`. Added `DiscNode` and typed `new_disc`. Added `AnyBox` dimension
helpers and retyped `append_to_vlist` so the legacy wrapper classifies `i32` to
`AnyBox` once at the boundary.

**Tests:** `node::` **25/25**. Dashboard green: `tex-outputs` **29/29**,
`trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**. Warnings stayed
**42**. Metrics unchanged: `with_state` 1197, `unsafe fn` 571, bare `p/q/r:
i32` params 379, direct `#[no_mangle]` grep 11.

**Next:** continue typed consumers now that rule/disc/ligature exist. Good
targets: more already-threaded packaging/math leaves that read box/rule dims,
or constructor families still raw (`new_math`, `new_glue`, lig-item/noad).

---


## Idiom session 153 — 2026-06-08 — Execution: box-family typed alloc adoption for `new_null_box`

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), the session-152
"Next" item: adopt the typed `alloc` pattern in **`new_null_box`**. **1 code
slice.** Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **18 → 19**.

### What changed

- Added a shared list-node header capability in `src/node.rs`:
  **`HasNodeHeader`** + `NodePtr<K>::subtype`/`set_subtype` for node families
  that really carry a `NODE_type`/subtype header. Implemented it for
  `HListNode`, `VListNode`, `GlueNode`, `KernNode`, and `PenaltyNode`.
  **Deliberately not implemented for `GlueSpecNode`** because its word 0 is
  `ref_count`/stretch-order/shrink-order, not a list-node header. This
  resolves session 152's "box needs `set_subtype`" handoff without exposing a
  false subtype on glue specs.
- Deduped the per-family subtype methods from `GlueNode`, `KernNode`, and
  `PenaltyNode`; existing call sites still use the same API, now through the
  shared header impl.
- Added **`NodePtr::<HListNode>::alloc(mem) -> Option<Self>`**: wraps
  `get_node(BOX_NODE_SIZE)`, writes `NODE_type = HLIST_NODE`, and confines the
  family construction to one audited `from_idx_unchecked` site. This is the
  box-family type-stamping allocator, still using TeX's free-list arena.
- Rewrote **`new_null_box`** (`src/mem.rs`) to allocate via
  `NodePtr::<HListNode>::alloc` and fill the box through typed setters:
  subtype, dimensions, nullable list pointer, glue sign/order/set. The legacy
  public surface still returns `i32`; `ALLOC_FAILED` propagation and the
  `_global` overflow wrapper are unchanged.

### Tests + validation

- Added `alloc_hlist_type_stamps_and_carves_a_usable_handle`, which builds a
  minimal free-list block, allocates an hlist, asserts it classifies back as
  `AnyBox::HList`, and verifies writable typed fields.
- Ran:
  - `cargo test -p tectonic_engine_xetex node:: --lib` → **19/19**
  - `cargo test -p tectonic --test tex-outputs` → **29/29**
  - `cargo test -p tectonic --test trip` → **2/2**
  - `cargo test -p tectonic --test engine_rust` → **7/7**
  - `cargo test -p tectonic --test latex_goldens` → **4/4**

### Metrics / handoff

Behaviour-preserving: each typed setter lowers to the same raw field write.
Metrics by the existing grep commands: `with_state` **1197**, `unsafe fn`
**571**, bare-`i32` node-ptr params **379**, `#[no_mangle]` direct grep **11**
(unchanged by this slice). New audited `unsafe` construction site: +1
(`HListNode::alloc`), matching the existing per-family `alloc` pattern.

**Next:** rank-5 **rule** remains the next family-build slice, but it needs a
running-dimension wrapper for `Scaled`-or-`NULL_FLAG` before `new_rule` can be
typed. The biggest live Phase-C typing win is still box-dimension-reading
leaves in `packaging.rs`/`shipout.rs`; `append_to_vlist` is already threaded
and wants `AnyBox` dimension helpers because its `b` can be hlist or vlist.

---


## Idiom session 152 — 2026-06-07 — Execution: extend the typed alloc pattern to kern + penalty — the FIRST type-stamping `alloc`s

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), the session-151
"Next" item: *extend the typed `alloc` pattern to kern/penalty/box when a
real allocating consumer threads*. **2 commits** (one family each, A.12).
Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **16 → 18**.

### What this session adds (and why it is new vs. 151)

Session 151 built the **first** typed allocator, `NodePtr::<GlueSpecNode>::
alloc`, but a glue_spec carries **no `NODE_type`**, so its `alloc` stamps
nothing. This session lands the **first type-stamping `alloc`s** — the
documented-but-never-exercised half of the `01c` §8 pattern ("a type-stamping
family's `alloc` additionally `set_b16s1(NODE_type)` before returning"):

- **`NodePtr::<KernNode>::alloc(mem) -> Option<Self>`** (`src/node.rs`):
  `get_node(MEDIUM_NODE_SIZE)`, **writes `NODE_type = KERN_NODE`**, then
  constructs the handle through the family's single audited
  `from_idx_unchecked`. `None` = the §5 arena-exhaustion sentinel.
- **`NodePtr::<PenaltyNode>::alloc`** (same shape, stamps `PENALTY_NODE`) +
  a small **`PenaltyNode::set_subtype`** (the shared-header b16.s0 half the C
  `new_penalty` zeroes; penalty has no meaningful subtype, doc'd as such).

### The two real consumers, now fully typed + `unsafe`-free

Both constructors were **already** threaded to `&mut MemState`, so this is
pure ptr/typing adoption (no threading work — the same shape as 151's
`math_glue`):

- **`new_kern`** (`src/mem.rs`): `let Some(p) =
  NodePtr::<KernNode>::alloc(mem) else { return ALLOC_FAILED };
  p.set_subtype(mem, NORMAL); p.set_width(mem, Scaled(w)); p.idx().0`.
  Fully typed, zero raw `mem.set_*` indexing, **`unsafe`-free** (it already
  was, but the hand-rolled `get_node` + raw `set_b16s1(KERN_NODE)` /
  `set_b16s0` / `set_b32s1(p+1)` are gone — confined into `alloc`).
- **`new_penalty`** (`src/mem.rs`): the analogous rewrite via
  `PenaltyNode::alloc` + `set_subtype(0)` + `set_penalty(m)`.

`KERN_NODE`/`PENALTY_NODE` imports **net-moved** from `mem.rs` into
`node.rs`'s `alloc`s (the type code now lives at the one stamping site);
`MEDIUM_NODE_SIZE` stays in `mem.rs` (other constructors still call
`get_node` directly).

### Tests + validation

Two new `node::` unit tests (16 → 18), each building a minimal single-free-
block arena and asserting (a) `alloc` carves a usable, writable handle and
(b) it **classifies back** (`classify_kern`/`classify_penalty` → `Some`),
which directly verifies the type-stamp the `glue_spec` test could not. Both
commits validated on the full dashboard (**29/2/7/4**) + the lib tests.

### Metrics

Behaviour-preserving (each typed setter/alloc lowers to the identical
`get_node` + `set_b16s*`/`set_b32s1`): `with_state` **1197**, `unsafe fn`
**571**, `#[no_mangle]` unchanged, bare-`i32` node-ptr params **379** (the
constructors still *return* `i32`, so no param metric moves this slice — the
win is the typed surface + the confined `from_idx_unchecked`). Net new
`unsafe` blocks: +2 (the two `alloc` audited sites), but they are the
single confined construction site per family — the pattern, not a leak.

### Lesson / handoff

The type-stamping `alloc` is now proven across two families. Per the `01c` §8
A.6 discipline, **do NOT** generalise to one `alloc<K>()` yet — but kern +
penalty + glue_spec means **three** families now share the shape, so the
generalisation question is getting closer (the blocker is that a single
`alloc<K>` needs a per-`K` "size + type-code" trait, which is the A.6
"≥2 real consumers" bar — revisit when box/rule adopt it). **Next:**
- **`new_null_box`** (box family) is the natural next `alloc` adoption, but
  it needs a box-family `set_subtype` (the box methods are trait-gated;
  `subtype` is the rank-0 *shared header* field — consider adding a single
  universal `subtype`/`set_subtype` on `NodePtr<K>` for **all** `K` and
  deduping the per-family copies on GlueNode/KernNode/PenaltyNode, but that
  is its own slice because a blanket `impl<K: NodeKind>` collides with the
  existing inherent per-family methods → method-resolution error unless they
  are removed together).
- **rank-5 rule family** (`new_rule` is its waiting consumer) — still the
  first sentinel-bearing family; needs the running-dimension wrapper
  (`Scaled`-or-`NULL_FLAG`) before `width`/`depth`/`height` can be typed.
- box-dimension-reading **leaves** in `packaging.rs`/`shipout.rs`
  (`append_to_vlist` is fully threaded; `b` is hlist-or-vlist → needs
  `AnyBox` dim helpers) remain the biggest §8 *param*-metric win.

---

## Idiom session 151 — 2026-06-07 — Execution: typesafe node allocation (`NodePtr::<GlueSpecNode>::alloc`) — confine `from_idx_unchecked`, remove the `math_glue` unsafe

**Type:** execution (workstream 01 / `01b` §3a + `01c` §8), prompted by a
design question: *"in `math_glue` you do an unsafe type assertion —
shouldn't there be a typesafe way to allocate a node of the correct type?
Isn't that a common usecase? What does the plan say?"* **1 commit.**
Dashboard **29/2/7/4**; warnings **42 → 42**; `node::` **16/16**.

### What the plan says (and the answer)

The plan **supports** a typed allocator and is explicit that the free-list
arena stays (`01c` §8 NON-GOAL: no per-node heap alloc; `03` perf rules).
The `unsafe { from_idx_unchecked }` at the `math_glue` call site was a
*symptom* — `math_glue` was hand-rolling the `get_node` + handle
construction that belongs in one audited place. `from_idx_unchecked` is
"unsafe" only as a **parity**-discipline marker (its own docs: not
memory-unsafe), so the fix is to **centralise** it in the allocator, not to
sprinkle it at call sites.

### The change

- **`NodePtr::<GlueSpecNode>::alloc(mem: &mut MemState) -> Option<Self>`**
  (`src/node.rs`): wraps the existing `get_node(GLUE_SPEC_SIZE)`, maps the
  `ALLOC_FAILED` sentinel to `None` (the §5 arena-exhaustion signal, handled
  by the caller after the borrow drops), and constructs the handle via the
  family's **single audited** `from_idx_unchecked`. Doc records the design
  rules: a node *being initialised* has no `NODE_type` to classify (so
  `from_idx_unchecked`, not `classify_*`, is correct here); a type-stamping
  family's `alloc` additionally `set_b16s1(NODE_type)` before returning
  (glue_spec carries no type → stamps nothing); it is the typed lift of
  TeX's `new_*` constructors, **not** a new allocator (free-list arena
  unchanged); do **not** generalise to one `alloc<K>()` until ≥2 families
  share it with real consumers (A.6). `01c` §8 + `01b` §3a updated.
- **`math_glue`** (`src/math.rs`) now: `let spec =
  NodePtr::<GlueSpecNode>::alloc(mem).ok_or(NodeRecycleAbort::Overflow)?;`
  then the five typed setters — **`unsafe`-free**. Removed the now-unused
  `GLUE_SPEC_SIZE` import. (The one remaining `from_idx_unchecked` in
  `math.rs` is the *justified* kern-classification boundary in
  `math_kern_global`, gated by its `KERN_NODE` match-arm caller.)
- **Unit test** (16th in `node::`): builds a minimal single-free-block arena
  (rover → self, `MAX_HALFWORD` free marker) and asserts `alloc` carves a
  usable, in-bounds, writable glue_spec handle — isolated coverage of the
  allocator wiring (`GLUE_SPEC_SIZE` + null check + construction), since
  `mem.rs` has no unit tests and otherwise relies on the parity suites.

### Why `Option`, not `Result<_, NodeRecycleAbort>`

`alloc` returns the most primitive, decoupled form — `Option<Self>`, the
typed lift of `get_node`'s `ALLOC_FAILED` — so `node.rs` does not depend on
`scan.rs`'s abort enum, and each caller chooses its abort policy
(`.ok_or(NodeRecycleAbort::Overflow)?` here).

### The companion question (subtype-as-enum) — deferred by design

Same turn asked whether `subtype` should be an enum instead of `u16` +
consts. Recorded the answer (not implemented): the plan mandates a checked
enum for the node *type* field and a second classification layer only for
subtypes that change *field layout* (whatsits). Plain tag subtypes stay
`u16` because **several are used arithmetically** — verified: glue node
`subtype = skip_param + 1` (`mem.rs:540,620`), recovered by subtraction — so
a blanket subtype enum is unfaithful (A.7) and lossy. A per-family tag enum
(`subtype_kind()`) on top of the raw accessor is fine **only** where the
subtype is a verified closed tag with no arithmetic use (kern qualifies;
glue does not). Opt-in polish, not a foundation requirement.

### Metrics & validation

- Behavior-preserving: `alloc` lowers to the identical `get_node` +
  index the old code emitted; `math_glue`'s writes are unchanged. Full
  dashboard green (commit touches the live `mlist_to_hlist` glue path).
- The raw `unsafe {}` block net-**moved** from `math.rs` into the one
  `node.rs` allocator site (confined, now reusable). `unsafe fn` /
  `#[no_mangle]` / `with_state` counts unchanged. `node::` 16/16.

**Handoff / Next:** unchanged from session 150 (more fused adoptions —
box-dimension readers in `packaging.rs`/`shipout.rs` are the biggest `01b`
§8 metric win; or build rank-5 **rule** with its running-dimension wrapper).
Plus: **extend the typed `alloc` pattern** to kern/penalty/box (each
stamping its `NODE_type`) the next time a real *allocating* consumer of
those families threads — do not add them speculatively (A.6).

## Idiom session 150 — 2026-06-07 — Execution: the FIRST fused `01b` §3a threading+typing adoptions (`math_kern`, `math_glue`) + glue_spec family fix

**Type:** execution (workstream 01 / `01b` §3a — the first adoptions of the
`01c` typed layer on live call sites). **3 commits.** Dashboard **29/2/7/4**
green on every commit; warnings **42 → 42**; `node::` **15/15**.

This is the milestone the foundation (sessions 145–149) was built for: the
typed `NodePtr<K>` layer now has **live consumers**, proving the
type-as-you-thread loop end-to-end — not just unit tests + the disasm gate.

### Commit 1 — `math_kern` → `NodePtr<KernNode>` (`f2e56c32`)

`math_kern` (`math.rs`) was already threaded to `&mut MemState` and is
reached only from `mlist_to_hlist`'s first-pass `KERN_NODE` match arm. The
fused touch:
- signature `math_kern(mem, p: i32, scaled_width: Option<i32>)` →
  `(mem, p: NodePtr<KernNode>, scaled_width: Option<Scaled>)`;
- body's two raw writes `mem.set_b32s1(p+1, w)` / `mem.set_b16s0(p, EXPLICIT)`
  → `p.set_width(mem, width)` / `p.set_subtype(mem, EXPLICIT)`;
- **classification boundary** = the `math_kern_global` bridge wrapper: its
  sole caller is the `KERN_NODE` arm (dispatches on `mem_b16s1(q) ==
  KERN_NODE`), so the node is already classified — `unsafe {
  NodePtr::<KernNode>::from_idx_unchecked(MemIdx::new(p)) }` with a SAFETY
  note citing that arm. The `Option<i32>` → `Option<Scaled>` conversion
  (`Scaled(scaled_mu_value(...))`) also lives in the wrapper.

`math.rs` is the first module to `use crate::node`/`crate::units`.

### Commit 2 — complete the glue_spec family (`2e7571a7`)

**Recon during commit 1/3 found the glue_spec family was missing its `width`
field** (+1.b32.s1). The session-147 field map (and thus the session-148
build) omitted it because `width` shares the generic `BOX_width` +1.b32.s1
offset — there is **no dedicated `GLUE_SPEC_width` C macro** — yet
`new_spec` (`mem.rs`) and `math_glue` (`math.rs`) read/write `mem[p+1]` as
the spec width. Added `NodePtr::<GlueSpecNode>::width`/`set_width` + extended
the round-trip test to cover +1. No new disasm run: the +1.b32.s1 `Scaled`
shape was already ICF-proven by box/kern `width`. Corrected the `01c` §1
table row + the family doc comment. (Foundation fix; `node.rs` is
`#[allow(dead_code)]` so it is behavior-neutral, but the full dashboard was
still run.)

### Commit 3 — `math_glue` → `NodePtr<GlueSpecNode>` (`64b80f1e`)

`math_glue` (`math.rs`, already `&mut MemState`-threaded) allocates a fresh
glue_spec via `get_node(GLUE_SPEC_SIZE)` and fills five fields. The fused
touch wraps the fresh index as `NodePtr<GlueSpecNode>` and routes all five
writes through typed setters (`set_width`/`set_stretch_order`/`set_stretch`/
`set_shrink_order`/`set_shrink`, each value `Scaled(...)`-wrapped where
scaled), returning `Ok(spec.idx().0)`. **Boundary lesson:** a *freshly
allocated* node has no `NODE_type` to classify yet, so this is the
"construct a handle to a node I am initializing" case — `from_idx_unchecked`
is the correct tool, **not** `classify_*`/`from_glue_ptr`. The closure rule
holds: every glue_spec access in the body goes through the one handle (no
mixed typed/raw indexing).

### Metrics & validation

- **Both adoptions are behavior-preserving**: each typed setter lowers to
  the identical `set_b32s1`/`set_b16s*` the raw code emitted (the Scaled
  wrap is value-identity). Verified by the full dashboard on each live-path
  commit (commits 1 + 3 touch `mlist_to_hlist`).
- **`01b` §8 metric:** bare-`i32` `[pqr]: i32` params (`grep -rhoE '\b[pqr]:
  i32\b'`) **380 → 379** — `math_kern`'s `p` retyped; the trend is now
  moving down, as intended. (`math_glue`'s spec is a local, not a `[pqr]`
  param, so it does not register on this proxy metric — but it removes 5 raw
  field pokes.)
- Warnings **42 → 42** (no unused imports — both `node`/`units` imports are
  consumed). `with_state`/`unsafe fn`/`#[no_mangle]` unchanged. `node::`
  15/15, `units::` 6/6.

**Handoff / Next:** the loop is proven; keep driving it. Two complementary
tracks:
1. **More fused adoptions.** The cheapest are functions **already** threaded
   to `&mut MemState` whose node-ptr param/local is a *single* known family.
   Grep `math.rs`/`packaging.rs`/`shipout.rs`/`linebreak.rs` for
   `fn …(mem: &(mut )?MemState, …, [pqr]: i32, …)` bodies that read/write
   one family's fields via raw `mem.bXXsY(p+N)`, and retype + swap in one
   commit. Box-dimension readers (the highest-traffic family) are the
   biggest metric win. Candidates seen this session: `math.rs`
   `stack_into_box`/`make_*_finish` (box `b`/`hbox` params),
   `append_script_box`. Always confirm the param is a *single* kind (a body
   that switches on `node_type` and handles several families can't take one
   `NodePtr<K>` — leave it raw or classify per-arm).
2. **Build rank-5 rule** (`01c` §1) — the first **sentinel-bearing** family:
   `width`/`depth`/`height` (+1/+2/+3.b32.s1) are `Scaled`-**or**-`NULL_FLAG`
   running dimensions (`NULL_FLAG = -0x40000000`, `xetex-constants.h:16`).
   Introduce a running-dimension wrapper (e.g. `enum RuleDimen { Running,
   Set(Scaled) }`) before typing the fields; mirror the exact `NULL_FLAG`
   compare; unit-test both running and set cases. Verify offsets against the
   `RULE_NODE_*` macros + `new_rule`. After rule: disc → ligature → math →
   native.

## Idiom session 222 — 2026-06-22 — Execution: thread most of `main_control_math_cases`

**Type:** execution (workstream 01 / Phase-C threading), intentionally light docs. Continued the inherited math WIP and finished a large mechanical slice of the math dispatcher.

### Change

- Kept the in-progress `app_display_in(&mut EngineState, ...)` and `check_math_font_params_in(&mut EngineState, ...)` cores usable from legacy `after_math`.
- Added `main_control_math_cases_in(&mut EngineState) -> Result<MathCaseAction, MainControlAbort>` and rewired all small/already-threaded arms to direct `_in` calls: scanner math readers/setters, `scan_math_in`, `start_eq_no_in`, `math_limit_switch_in`, `math_radical_in`, `math_ac_in`, `append_choices_in`, `sub_sup_in`, `math_fraction_in`, `math_left_right_in`, `off_save_in`, and the `VCENTER` arm.
- Deleted now-dead bridge wrappers/accessors exposed by the dispatcher threading across `errors.rs`, `globals.rs`, `math.rs`, `mem.rs`, `packaging.rs`, and `scan.rs`.

### Validation

- `cargo fmt -p tectonic_engine_xetex`
- `cargo check -p tectonic_engine_xetex` — warnings **41**
- Dashboard: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**
- Release rebuild: `cargo build --release -p tectonic`
- Full corpus: **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`

**Handoff / Next:** remaining math dispatcher fallbacks are `init_math` and `after_math`. Thread those next, then collapse `MathCaseAction` away and make `main_control_math_cases_in` return plain `Result<(), MainControlAbort>`.

## Idiom session 224 — 2026-06-22 — Execution: thread `open_or_close` and `issue_message`

**Type:** execution (workstream 01 / Phase-C threading), intentionally light docs. Used one explorer subagent for the `open_or_close` dependency audit; the actual edits stayed local.

### Change

- `io::open_or_close` now has a real `open_or_close_in(&mut EngineState) -> Result<(), OpenOrCloseAbort>` core. It uses threaded scanner/file-name/open helpers and direct `st.input`/`st.eqtb` access; the no-arg function is now only the bridge wrapper.
- `output::issue_message` now has `issue_message_in(&mut EngineState) -> Result<(), IssueMessageAbort>`, using threaded token scanning, output, string-pool, and error paths under one borrow.
- Deleted dead bridge readers exposed by those slices: read-stream readers, UTF-16 filename readers, and the old `term_offset`/`file_offset`/`long_help_seen` readers.

### Validation

- `cargo fmt -p tectonic_engine_xetex`
- `cargo check -p tectonic_engine_xetex` — warnings **41**
- Dashboard: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**
- Release rebuild: `cargo build --release -p tectonic`
- Full corpus: **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`

**Handoff / Next:** continue the mode-independent frontier with `show_whatever` and `do_extension`. Do not single-borrow `main_control_mode_independent` until those legacy arms are threaded; otherwise it will re-enter `with_state`.

## Idiom session 225 — 2026-06-22 — Execution: thread `show_whatever`, prep `new_write_whatsit`

**Type:** execution (workstream 01 / Phase-C threading), intentionally light docs. Used one explorer subagent for the `show_whatever` / `do_extension` audit; edits stayed local.

### Change

- `output::show_whatever` now has `show_whatever_in(&mut EngineState) -> Result<(), ShowWhateverAbort>`, using threaded scanner/output/error helpers and direct `st.mem`/`st.eqtb`/`st.scanner` access under one borrow.
- `ext::new_write_whatsit` now has `new_write_whatsit_in(&mut EngineState, ...) -> Result<(), NewWriteWhatsitAbort>`, giving future `do_extension_in` a ready building block.
- Deleted dead bridge wrappers/accessors exposed by the slice: `scan_four_bit_int`, `scan_register_num`, `the_toks`, `flush_list_global`, `find_sa_element_global`, several output show/print wrappers, and old `error_count`/`if_limit` readers.

### Validation

- `cargo fmt -p tectonic_engine_xetex`
- `cargo check -p tectonic_engine_xetex` — warnings **41**
- Dashboard: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**
- Release rebuild: `cargo build --release -p tectonic`
- Full corpus: **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`

**Handoff / Next:** `do_extension` remains the next mode-independent frontier, but full threading needs the picture/native/encoding blockers handled first (`load_picture`, `measure_native_glyph_global`, encoding helpers). Avoid single-borrowing `main_control_mode_independent` until `do_extension` is bridge-free.

## Idiom session 226 — 2026-06-22 — Execution: thread `do_extension` and picture loading

**Type:** execution (workstream 01 / Phase-C threading), intentionally light docs. Used one explorer subagent for the `do_extension` dependency audit; edits stayed local.

### Change

- Added `ext::do_extension_in(&mut EngineState) -> Result<(), MainControlAbort>` and rewired the write/special/immediate/language/timer/random/glyph/encoding arms to threaded scanner/input/output/shipout helpers plus direct `st.mem`/`st.eqtb`/`st.scanner` access.
- Added `pic::load_picture_in(&mut EngineState, ...) -> Result<(), MainControlAbort>`, including a borrow-safe `find_pic_file` path that takes the packed filename pointer instead of reading `name_of_file()` under a live borrow.
- Added `main_control_mode_independent_in(&mut EngineState)` over the now-threaded arms. The top `main_control` loop still calls the legacy wrapper because that loop itself is not yet threaded.
- Deleted dead bridge wrappers/accessors exposed by the slice across `scan`, `output`, `ext`, `pic`, `shipout`, `save`, `errors`, and `globals`.

### Validation

- `cargo fmt -p tectonic_engine_xetex`
- `cargo check -p tectonic_engine_xetex` — warnings **41**
- Dashboard: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**
- Release rebuild: `cargo build --release -p tectonic`
- Full corpus: **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`
- Metrics: `with_state` matches **629**; `unsafe fn` matches **357**

**Handoff / Next:** continue upward into the remaining legacy `main_control` loop call sites. `main_control_mode_independent_in` is ready for a threaded caller; the top loop is still the boundary.

## Idiom session 227 — 2026-06-22 — Execution: thread main-control box/list dispatchers

**Type:** execution (workstream 01 / Phase-C threading), intentionally light docs. Used two explorer subagents for target scouting; edits stayed local.

### Change

- Added `_in(&mut EngineState)` cores for mode-error cases, simple-list group cases, spacer/ignore-spaces, normal-space append, inter-char input execution, and the large `main_control_box_list_cases` dispatcher.
- Rewired the dispatcher to threaded alignment/box/pagebuilder/list helpers and direct `st.*` access; `make_accent` remains a deferred action so the wrapper can drop the borrow first.
- Deleted exposed dead bridge wrappers/accessors across `main_control`, `errors`, `input`, `pagebuilder`, `save`, `scan`, `mem`, and `globals`.

### Validation

- `cargo fmt -p tectonic_engine_xetex`
- `cargo check -p tectonic_engine_xetex` — warnings **42**
- Dashboard: `tex-outputs` **29/29**, `trip` **2/2**, `engine_rust` **7/7**, `latex_goldens` **4/4**
- Release rebuild: `cargo build --release -p tectonic`
- Corpus: document scope **36 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1` and `1905.00148v1`; LVT scope **601 passed / 0 diverged / 0 missing-golden / 0 timed-out** via a temporary progress runner after uninstrumented `check all` stayed silent in the LVT tail. Combined baseline unchanged: **637 passed / 2 diverged / 0 missing-golden**.
- Metrics: `with_state` matches **534**; `unsafe fn` matches **326**.

**Handoff / Next:** thread `make_accent` or the top `main_control` loop boundary. The box/list, mode-independent, alignment, and math dispatchers are now ready for a threaded caller.
