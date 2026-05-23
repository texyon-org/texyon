# Idiom session log

Append-only log for Stage 2 (idiomatization). Newest entry at the
**top**, under this header. Distinct from the Stage 1 `../SESSION_LOG.md`
(translation), which is closed.

Each session: what it did (plan construction or execution), the test
dashboard if code changed, surprises, and a handoff.

---

## Idiom session 8 — 2026-05-23 — scanner file-name group Phase-B bridge

**Type:** execution (workstream 01).

**Goal:** migrate a coherent, self-contained, non-hot `scanner` scalar
group, and exercise the new deferred-newtype-worklist rule for the first
time.

**Work done:**
- Added `ScannerState` under `EngineState` and moved the file-name
  scanning group into it: `cur_name`, `cur_area`, `cur_ext`,
  `area_delimiter`, `ext_delimiter`, `quoted_filename`,
  `file_name_quote_char`, `name_in_progress`, `stop_at_space`.
- Repointed the `globals.rs` accessors at `state::with_state` /
  `with_state_mut`; removed the raw externs and the C definitions in
  `xetex/xetex-ini.c`.
- Recorded the newtype debts created by this move in the Deferred
  newtype worklist (`01a-global-inventory.md`): `cur_name/area/ext` →
  `StrNumber`, `area_delimiter`/`ext_delimiter` → `PoolPtr`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 326 → 317.
- Raw `unsafe { ... }` text-search count: 739 → 721.
- `globals.rs`: 67 globals now state-backed.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- Chose a non-hot scanner sub-group (file-name parsing) on purpose: the
  truly hot scanner scalars (`cur_cmd`, `cur_chr`, `cur_tok`, `cur_cs`,
  `cur_val`) are touched on essentially every `get_next`/`get_token`,
  so routing them through the thread-local `RefCell` bridge risks a
  measurable slowdown. Defer those until Phase C threads `&mut` instead,
  or benchmark first.
- `name_in_progress` / `quoted_filename` / `stop_at_space` / the
  `cur_*` defs were scattered across `xetex-ini.c` (lines 46, 139–147,
  291), not contiguous — each needed individual removal.
- Verified (grep) no non-macOS C reader and no separate `extern`
  redeclaration in other Rust modules before moving.

**For the next session:**
- The remaining easy scalar wins are mostly in `scanner` (non-hot
  groups) and small `eqtb`/`save_stack` scalar sets. The ownership-
  bearing clusters (`strings`, `fonts` arrays, the `shipout`
  file-handles) still need single ownership-aware slices that also fix
  `tt_cleanup`/`#[link_name]` paths.
- Keep `with_state` closures single-field; do not call translated engine
  code while a borrow is live.

## Idiom session 7 — 2026-05-23 — shipout scalar cluster Phase-B bridge

**Type:** execution (workstream 01).

**Goal:** continue Phase B by moving the remaining low-coupling
`shipout` scalar globals into `EngineState.shipout`, building on the
tiny session-6 slice, without taking on the pointer/file-handle
ownership surface of that cluster.

**Work done:**
- Extended `ShipoutState` with 21 scalar/array fields and repointed
  their `globals.rs` accessors at `state::with_state` /
  `with_state_mut`: `selector`, `doing_special`, `term_offset`,
  `file_offset`, `old_setting`, `epochseconds`, `microseconds`,
  `dead_cycles`, `page_tail`, `last_badness`, `page_contents`,
  `page_so_far` (`[i32; 8]`), `last_glue`, `last_penalty`, `last_kern`,
  `last_node_type`, `insert_penalties`, `output_active`,
  `pdf_last_x_pos`, `pdf_last_y_pos`, and `insert_src_special_auto`.
- Removed the matching raw `extern` declarations from `globals.rs`
  `mod raw` and the C definitions from `xetex/xetex-ini.c`.
- Left `disc_ptr` (scanner cluster) untouched where it was interleaved
  with the page-builder scalars.

**Metrics after this session:**
- `globals.rs` raw extern globals: 347 → 326.
- Raw `unsafe { ... }` text-search count: 780 → 739.
- `globals.rs`: 58 globals now state-backed.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- These globals are only reached through `globals.rs` accessors: no
  non-macOS C reader and no separate `extern` redeclaration in other
  Rust modules (verified by grep), so the move is purely repoint +
  delete-def. Print-path scalars (`selector`, `file_offset`,
  `term_offset`) are consistent with the errors-cluster print scalars
  already bridged in session 5.
- None of these had `#[link_name]` cleanup aliases, so no `ini.rs`
  free-path edits were needed.
- The remaining `shipout` rows are pointer / file-handle state
  (`rust_stdout`, `log_file`, `write_file`, `current_diagnostic`) that
  carry init/cleanup edges; defer them to an ownership-aware slice.

**For the next session:**
- Either take an ownership-bearing cluster (`strings` is the planned
  next big one: `str_start` / `str_pool` allocation, cleanup aliases,
  format load/dump), or finish the remaining `shipout` file-handle
  state as its own ownership-aware slice.
- Keep `with_state` / `with_state_mut` closures single-field; do not
  call translated engine code while a `RefCell` borrow is live.

## Idiom session 6 — 2026-05-23 — tiny shipout source-special flag bridge

**Type:** execution (workstream 01, short slice).

**Goal:** make one small non-`misc` state migration without taking on
the `strings` ownership-transfer surface.

**Work done:**
- Added `ShipoutState` under the thread-local `EngineState`.
- Moved three source-special flags behind `EngineState.shipout`:
  `insert_src_special_every_par`, `insert_src_special_every_math`, and
  `insert_src_special_every_vbox`.
- Repointed their `globals.rs` getters to `state::with_state`.
- Removed the old C definitions from `xetex/xetex-ini.c`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 350 → 347.
- Raw `unsafe { ... }` text-search count: 783 → 780.
- `globals.rs` wrappers: 815 total, with 37 globals now state-backed.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- The other dashboard suites were not rerun in this short session; they
  passed in session 5 after the larger `errors` migration.

**Surprises / gotchas:**
- These flags are currently read from translated Rust but not set by
  in-scope code; the migration preserves the effective default `false`
  behavior.
- This was deliberately kept smaller than `strings`, which still needs a
  single ownership-aware slice covering allocation, cleanup aliases, and
  format load/dump paths.

**For the next session:**
- Either take the planned `strings` ownership-bearing cluster with full
  validation, or continue with another small non-`misc` scalar group
  that has no cleanup aliases.

## Idiom session 5 — 2026-05-23 — errors cluster Phase-B bridge

**Type:** execution (workstream 01).

**Goal:** continue Phase B by moving the `errors` inventory cluster into
the Rust-owned `EngineState` bridge without changing engine behavior.

**Work done:**
- Extended `src/state.rs` with `ErrorState`, including scalar state,
  small fixed arrays, and the `help_line` pointer array.
- Moved 27 `errors` globals behind `EngineState.errors`:
  `arith_error`, `tex_remainder`, `dig`, `randoms`, `j_random`,
  `help_line`, `help_ptr`, `two_to_the`, `spec_log`, `interaction`,
  `history`, `error_count`, `use_err_help`, `long_help_seen`,
  `log_opened`, `tally`, `error_line`, `half_error_line`, `trick_buf`,
  `trick_count`, `first_count`, `deletions_allowed`, `random_seed`,
  `no_shrink_error_yet`, `font_in_short_display`, `depth_threshold`,
  and `breadth_max`.
- Repointed the existing `globals.rs` accessors at
  `state::with_state` / `with_state_mut`, adding setters for
  `error_line` and `half_error_line` so initialization no longer needs
  `#[link_name]` aliases.
- Removed the old C definitions from `xetex/xetex-ini.c` and
  `xetex/xetex-linebreak.c`.
- Removed the `src/ini.rs` aliases for `error_line` /
  `half_error_line` and initialized those values through the new
  accessors.

**Metrics after this session:**
- `globals.rs` raw extern globals: 377 → 350.
- Raw `unsafe { ... }` text-search count: 835 → 783.
- `globals.rs` wrappers: 815 total, with 34 globals now state-backed.

**Validation:**
- Baseline before change:
  - `cargo test -p tectonic --test tex-outputs`: 29/29.
  - `cargo test -p tectonic --test trip`: 2/2.
  - `cargo test -p tectonic --test engine_rust`: 7/7.
  - `cargo test -p tectonic --test latex_goldens`: 4/4.
- After change:
  - `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
  - `cargo test -p tectonic --test tex-outputs`: 29/29.
  - `cargo test -p tectonic --test trip`: 2/2.
  - `cargo test -p tectonic --test engine_rust`: 7/7.
  - `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- This crate's Rust toolchain does not derive `Default` for arrays as
  large as `[i32; 55]` and `[u16; 256]`, so `ErrorState` needs an
  explicit `Default` implementation.
- `no_shrink_error_yet` lived in `xetex/xetex-linebreak.c`, not the
  main init C file, confirming that each inventory row needs its old C
  definition checked individually.
- Running several Cargo tests concurrently is safe but noisy because
  they serialize on package/build locks.

**For the next session:**
- Continue workstream 01 with another non-`misc` cluster. `strings` is a
  logical next target, but it has ownership-transfer risk: handle
  `str_start` / `str_pool` allocation, format load/dump behavior, and
  `tt_cleanup` aliases in the same slice.
- Keep `with_state` / `with_state_mut` closures as single-field reads
  and writes; do not call translated engine code while the `RefCell`
  borrow is live.

## Idiom session 4 — 2026-05-23 — config cluster Phase-B bridge

**Type:** execution (workstream 01, first code refactor).

**Goal:** start Phase B with a least-coupled non-`misc` cluster and prove
the `EngineState` bridge shape on real state.

**Work done:**
- Added `src/state.rs` with a minimal thread-local `EngineState` and
  `ConfigState`.
- Moved the seven `config` globals behind `EngineState.config`:
  `file_line_error_style_p`, `halt_on_error_p`, `in_initex_mode`,
  `synctex_enabled`, `semantic_pagination_enabled`,
  `shell_escape_enabled`, and `max_print_line`.
- Repointed the existing `globals.rs` accessors at
  `state::with_state` / `with_state_mut`, adding setters/getters needed
  to replace the previous `#[link_name]` access.
- Removed the old C definitions in `xetex/xetex-ini.c` for those seven
  symbols.
- Removed the `src/ini.rs` `#[link_name]` aliases for
  `file_line_error_style_p` and `max_print_line`, and replaced the
  direct `synctex_enabled` extern in `src/synctex.rs` with the config
  accessor.

**Metrics after this session:**
- `globals.rs` raw extern globals: 384 → 377.
- Raw `unsafe { ... }` text-search count: 846 → 835.
- `globals.rs` wrappers: 813 total, with 7 now state-backed instead of
  C-global-backed.

**Validation:**
- Baseline before change:
  - `cargo test -p tectonic --test tex-outputs`: 29/29.
  - `cargo test -p tectonic --test trip`: 2/2.
  - `cargo test -p tectonic --test engine_rust`: 7/7.
  - `cargo test -p tectonic --test latex_goldens`: 4/4.
- After change:
  - `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
  - `cargo test -p tectonic --test tex-outputs`: 29/29.
  - `cargo test -p tectonic --test trip`: 2/2.
  - `cargo test -p tectonic --test engine_rust`: 7/7.
  - `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- `cargo check -p tectonic_engine_xetex` needed escalated filesystem
  permissions because Cargo wanted to write the workspace target lock
  outside the crate writable root.
- `synctex.rs` had a direct Rust extern read of `synctex_enabled` that
  bypassed `globals.rs`; this is exactly the kind of side edge each
  inventory row needs checked for during cluster migration.
- `cargo fmt` reformatted a large `ini.rs` import list; unrelated
  `scan.rs` wrapping was removed from the final diff.

**For the next session:**
- Continue workstream 01 with another least-coupled non-`misc` cluster.
  `errors` is a good next target if you want scalar/array state without
  ownership transfer; `strings` is the next ownership-bearing target but
  needs careful handling of `str_start` / `str_pool` allocation and
  `tt_cleanup` aliases.
- Keep Phase-B bridge accessors short-lived. Do not call translated
  engine code while a `RefCell` borrow from `with_state` is live.

## Idiom session 3 — 2026-05-23 — global inventory created

**Type:** plan construction/validation (plan docs only; no code changes).

**Goal:** create the `01a-global-inventory.md` deliverable required
before executing workstream 01.

**Work done:**
- Generated and reviewed `01a-global-inventory.md` from:
  - `src/globals.rs` raw extern declarations and accessor bodies.
  - Promoted Rust `pub static mut` definitions in `src/*.rs`.
  - Non-macOS C files compiled by `build.rs`.
  - `src/ini.rs` `#[link_name]` aliases used by init/load/cleanup glue.
- Inventory scope:
  - 384 raw extern globals in `globals.rs`.
  - 16 promoted Rust `pub static mut` globals outside `globals.rs`.
  - 18 additional private module-local `static mut` values in an
    appendix, because they are also mutable engine state.
- Confirmed macOS remains intentionally out of scope and is not an
  inventory blocker.
- Marked workstream 01 `READY`, with the constraint that `misc` rows
  are a holding area and must not be migrated until their final
  sub-structs are decided.

**Test dashboard:** unchanged (docs only) — not rerun.

**For the next session:**
- Start executing workstream 01 on a least-coupled non-`misc` cluster.
  Recommended first choices: `strings`, `errors`, or `config`.
- For each moved symbol, use `01a-global-inventory.md` to update old C
  definitions, Rust `#[link_name]` aliases/free paths, and accessors in
  the same commit.

## Idiom session 2 — 2026-05-23 — plan review hardening

**Type:** plan construction/review (plan docs only; no code changes).

**Goal:** thoroughly review the idiomatization plan before execution and
patch inaccuracies or missing hazards.

**Work done:**
- Re-checked headline metrics against the current source:
  - `src/globals.rs` has 384 `pub(super) static mut` extern globals and
    811 accessor functions.
  - `src/*.rs` has 576 `#[no_mangle]` exports, 694 `unsafe fn` items,
    and 846 raw `unsafe { ... }` blocks by text search. The clippy
    undocumented-unsafe bucket remains recorded as 688.
  - `scan.rs` / `globals.rs` / `ini.rs` line counts still match
    18,748 / 5,569 / 4,768.
- Corrected the "remaining C" description: in addition to macOS and
  `yhash`, `xetex-engine-interface.c` still contains the C `setjmp`
  entry shim plus the aarch64 linker hack.
- Tightened ownership language: current arrays are allocated by Rust
  code through C allocator helpers and freed by `tt_cleanup`; they are
  not Rust-owned until the `Vec`/`EngineState` move. Each cluster move
  must update old C symbols and cleanup aliases in the same commit.
- Added the key type-taxonomy correction: `MemIdx` is only for valid
  arena indexes; TeX halfwords that may contain `TEX_NULL` need a
  separate `MemPtr`/`Halfword` wrapper. Also corrected `FontId` to be
  `i32`-backed initially (`internal_font_number` is `int32_t`), banned
  unchecked `NodeType` transmute, and deferred broad `Scaled` operator
  overloads until arithmetic side effects are audited.
- Added Phase-B `RefCell` borrow guidance so thread-local accessors do
  not call legacy code while holding a borrow.
- Recorded the explicit scope decision that `xetex-macos.c` must not be
  translated or used as a blocker for this Linux-validated plan.

**Test dashboard:** unchanged (no code touched) — not rerun.

**For the next session:**
- Produce `01a-global-inventory.md`. It must include cluster, current
  type, old definition location, promoted-static status, C
  reader/writer/linker status for non-macOS compiled C, `#[link_name]`
  cleanup alias, and known accessors for every global. Do not mark
  workstream 01 `READY` until that inventory exists. macOS-only
  references should be noted as intentionally out of scope.

## Idiom session 1 — 2026-05-23 — Stage 2 scaffolding + workstream 01 draft

**Type:** plan construction (no code changes; no commits).

**Goal:** stand up the idiomatization plan directory and draft the first
(highest-leverage) workstream so future sessions can pick up cold.

**Work done:**
- Surveyed the end-of-Stage-1 state. Key findings, now recorded in
  `00-overview.md`:
  - The crate is stuck at "Phase A" of `../02-state.md`: ~385 `static
    mut` globals, ~810 thin accessors in a 5,569-LOC `globals.rs`, ~580
    `#[no_mangle] pub unsafe extern "C"` internal fns, ~694 `unsafe fn`,
    raw `i32` arena indices, `scan.rs` at 18,748 LOC.
  - `cargo clippy -p tectonic_engine_xetex` = **694 errors + 358
    warnings**. 688 of the 694 errors are `undocumented_unsafe_blocks`
    (denied at the workspace level, `Cargo.toml:70`). The other 6 are
    correctness lints (`never_loop` ×2, `absurd_extreme_comparisons` ×2,
    `reversed_empty_ranges` ×1) that are probably faithful C quirks →
    `#[allow]` with justification, not "fixes".
  - **No in-scope C reads engine state** (no `mem[`/`eqtb[` outside
    macos), and `mem` is already Rust-allocated (`src/ini.rs:2669`,
    `:3258`). So the state-ownership move needs no C/Rust sync shim — it
    is a Rust-internal refactor.
- Asked the user two directional questions. Decisions (also in
  `STATUS.md`): end-state = **full Phase C**; first workstream =
  **state & memory redesign**.
- Wrote: `README.md`, `00-overview.md`, `STATUS.md`, this log, and
  `01-state-and-memory.md` (DRAFTED).
- Captured the **longjmp vs. Drop** hazard as a first-class design
  constraint (state must stay in a `'static`/`ManuallyDrop` anchor across
  the C `setjmp` boundary; thread `&mut` below it).

**Test dashboard:** unchanged (no code touched) — 29 / 2 / 7 / 4.

**Surprises / gotchas:**
- The biggest clippy bucket is a *symptom*, not 688 problems. Recorded an
  explicit anti-task: do **not** bulk-add SAFETY comments before the
  state migration dissolves the unsafe.
- `PROGRESS.md`'s "functions migrated" table has some stale `~??` rows
  from Stage 1; not worth fixing — Stage 2 tracks its own metrics in
  `idiom/STATUS.md`.

**For the next session:**
- Bring `01-state-and-memory.md` to `READY`:
  1. Validation pass — re-verify the global counts and the
     `globals.rs` structure references against current source.
  2. Produce the **`01a-global-inventory.md`** deliverable (§3 of the
     01 plan): the full global→cluster mapping table. This is itself a
     good, low-risk execution-style session and unblocks Phase B.
  3. Decide the open questions in §9 (newtype rollout (a) vs (b);
     EngineState anchor vs. setjmp).
- Then either start executing 01's first sub-phase (arena `Mem` + the
  `src/mem.rs` allocators, which have tight golden coverage) or draft
  `02-modules.md`.
- Cheap clippy wins (workstream 04) remain available as low-risk filler;
  do not touch the unsafe-block bucket yet.
