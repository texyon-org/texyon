# Idiomatization status

The authoritative tracker for Stage 2. Two sections: **plan
construction** (are the workstream plans written and ready?) and
**execution** (are the refactors done?). Update at every session end.

Last updated: idiom session 13 (2026-05-23).

## How to read this file

A workstream plan moves through these construction states:

-   `PLANNED` — listed, not yet drafted.
-   `DRAFTING` — being written this session.
-   `DRAFTED` — written but not yet self-reviewed/validated against the
    code.
-   `READY` — drafted **and** spot-checked against the current source
    (paths, line numbers, counts verified); safe to execute.
-   `EXECUTING` — a session is refactoring against it.
-   `DONE` — the workstream's refactor is complete and on `master`.

**Execution may only begin on a `READY` plan** (hard rule 4 in
`README.md`).

## Plan construction

| # | Workstream plan | State | Notes |
|---|---|---|---|
| — | `README.md` | DRAFTED | Navigation + hard rules; reviewed for remaining-C precision. |
| — | `00-overview.md` | DRAFTED | Diagnosis, target, guardrails, sequencing; reviewed and tightened. |
| — | `STATUS.md` | DRAFTED | This file. |
| — | `SESSION_LOG.md` | DRAFTED | Append-only; first entry written. |
| 01a | `01a-global-inventory.md` | READY | Exhaustive global inventory: 384 raw extern globals, 16 promoted Rust statics, 18 private module-local statics appendix. |
| 01 | `01-state-and-memory.md` | READY | First/highest-leverage workstream. Execute least-coupled non-`misc` clusters first; do not migrate `misc` rows until their final sub-structs are decided. |
| 02 | `02-modules.md` | PLANNED | Module decomposition. Draft after 01's cluster boundaries are settled. |
| 03 | `03-deunsafe-ffi.md` | PLANNED | De-unsafe + FFI teardown. Mostly falls out of 01. |
| 04 | `04-clippy.md` | PLANNED | Cheap clippy wins now; unsafe-block bucket after 01/03. |
| 05 | `05-errors-and-misc.md` | PLANNED | Error-handling shape, dead-comment cleanup, longjmp/Drop. Partly optional. |

## Execution

Workstream 01 is executing. Phase-B bridge has started with
least-coupled non-`misc` clusters:

-   `config` moved into `EngineState.config`:
    `file_line_error_style_p`, `halt_on_error_p`, `in_initex_mode`,
    `synctex_enabled`, `semantic_pagination_enabled`,
    `shell_escape_enabled`, `max_print_line`.
-   Old C symbol definitions and Rust `#[link_name]` aliases for the
    moved config globals were removed/redirected in the same change.
-   `errors` moved into `EngineState.errors`: `arith_error`,
    `tex_remainder`, `dig`, `randoms`, `j_random`, `help_line`,
    `help_ptr`, `two_to_the`, `spec_log`, `interaction`, `history`,
    `error_count`, `use_err_help`, `long_help_seen`, `log_opened`,
    `tally`, `error_line`, `half_error_line`, `trick_buf`,
    `trick_count`, `first_count`, `deletions_allowed`, `random_seed`,
    `no_shrink_error_yet`, `font_in_short_display`,
    `depth_threshold`, and `breadth_max`.
-   Old C definitions in `xetex/xetex-ini.c` /
    `xetex/xetex-linebreak.c` and the `src/ini.rs` `#[link_name]`
    aliases for `error_line` / `half_error_line` were removed in the
    same change.
-   Tiny `shipout` source-special flag slice moved into
    `EngineState.shipout`: `insert_src_special_every_par`,
    `insert_src_special_every_math`, and
    `insert_src_special_every_vbox`.
-   `shipout` scalar cluster moved into `EngineState.shipout` (21 more
    fields): `selector`, `doing_special`, `term_offset`, `file_offset`,
    `old_setting`, `epochseconds`, `microseconds`, `dead_cycles`,
    `page_tail`, `last_badness`, `page_contents`, `page_so_far`,
    `last_glue`, `last_penalty`, `last_kern`, `last_node_type`,
    `insert_penalties`, `output_active`, `pdf_last_x_pos`,
    `pdf_last_y_pos`, and `insert_src_special_auto`. Raw externs and C
    definitions removed in the same change; no cleanup aliases existed.
    Remaining `shipout` rows are pointer/file-handle state with
    init/cleanup edges (`rust_stdout`, `log_file`, `write_file`,
    `current_diagnostic`).
-   `scanner` file-name group moved into `EngineState.scanner` (new
    `ScannerState`): `cur_name`, `cur_area`, `cur_ext`,
    `area_delimiter`, `ext_delimiter`, `quoted_filename`,
    `file_name_quote_char`, `name_in_progress`, `stop_at_space`. Raw
    externs and C defs removed; no cleanup aliases. Newtype debts
    (`cur_*` → `StrNumber`, `*_delimiter` → `PoolPtr`) recorded in the
    inventory worklist. The hot scanner scalars (`cur_cmd`, `cur_chr`,
    `cur_tok`, `cur_cs`, `cur_val`) are intentionally deferred to avoid
    a thread-local cost on the tokenizer hot path.
-   `save_stack` scalar cluster moved into `EngineState.save_stack` (new
    `SaveStackState`): `save_ptr`, `max_save_stack`, `cur_level`,
    `cur_group`, `cur_boundary`. The `save_stack` pointer and `save_size`
    capacity stay behind (ownership/cleanup edges) for a later slice.
    Raw externs and C defs removed; no cleanup aliases touched.
    `cur_group` → `GroupCode` recorded in the worklist.
-   `eqtb` scalar set moved into `EngineState.eqtb` (new `EqtbState`):
    `hash_used`, `hash_high`, `no_new_control_sequence`, `prim_used`,
    `cs_count`. The `eqtb`/`hash`/`prim` arrays and the alias-bearing
    `hash_extra`/`eqtb_top`/`hash_top` stay behind for the ownership-
    aware `eqtb` array slice. Raw externs and C defs removed; no cleanup
    aliases touched. No newtype debts (all plain int/bool).
-   **First ownership move:** the `save_stack` *array* moved from a
    C-`malloc`'d pointer into a Rust-owned `Vec<MemoryWord>` on
    `EngineState.save_stack.data`. Accessors index the `Vec` (safe,
    bounds-checked; only the `MemoryWord` union projection stays
    `unsafe`); `ini.rs` allocates the `Vec` at init and `tt_cleanup`
    drops it; both `#[link_name="save_stack"]` aliases, the raw extern,
    and the C def were removed in the same commit. `save_size` capacity
    stays a C global. Establishes the §5 arena pattern.
-   **First `input`-cluster ownership move:** the `param_stack` *array*
    moved into a Rust-owned `Vec<i32>` on
    `EngineState.input.param_stack` (new `InputClusterState` sub-struct).
    Accessors index the `Vec` (safe, bounds-checked, no `unsafe`);
    `ini.rs` allocates the `Vec` (`param_size + 1`) at init and
    `tt_cleanup` drops it; **both** `#[link_name="param_stack"]` aliases
    (alloc `tre_*` + cleanup `tt_cleanup_*`), the raw extern, and the C
    def were removed in the same commit. `param_ptr`, `max_param_stack`,
    `param_size` stay C globals. Not in format dump. No newtype debt
    (plain `int32_t *`).
-   **Input line/group stacks (5 arrays):** `line_stack`, `grp_stack`,
    `if_stack`, `source_filename_stack`, `full_source_filename_stack`
    moved from C-`malloc`'d pointers into Rust-owned `Vec<i32>` on
    `EngineState.input` (same session-12 arena shape). All sized
    `max_in_open + 1`; allocated in one `ini.rs` block, dropped in
    `tt_cleanup`. The explicit slot-0 init writes were rewritten to the
    new `set_*` accessors. Both `#[link_name]` aliases per array (10
    decls), the 5 raw externs, and the 5 C defs (two blocks in
    `xetex-ini.c`) were removed in the same commit. Not in format dump.
    Newtype debts recorded: `grp_stack` (`save_pointer`),
    `source_filename_stack`/`full_source_filename_stack` (`str_number`);
    `line_stack`/`if_stack` are plain `int32_t` (no row). `eof_seen`
    (`*mut bool`) is the last simple input array, left for a later slice.

### Idiom metrics (baseline = session 132, end of Stage 1)

These trend toward their targets as workstreams execute. Re-measure with
the snippets in `00-overview.md` / the workstream plans.

| Metric | Baseline (2026-05-23) | Target | Current |
|---|---:|---|---:|
| `extern static mut` globals (`globals.rs`) | 384 | 0 (non-const) | 300 |
| promoted Rust `pub static mut` globals | 16 | 0 (non-const) | 16 |
| thin global accessors (`globals.rs`) | 811 | 0 | 815 total / 77 state-backed scalars + 7 Rust-owned arenas (save_stack, param_stack, line/grp/if/source_filename/full_source_filename stacks) |
| `#[no_mangle]` exports | 576 | boundary-only | 576 |
| `unsafe fn` (crate-wide) | 694 | boundary-only | 694 |
| raw `unsafe { ... }` blocks | 846 text-search / 688 clippy-documented bucket | boundary-only | 683 / 688 not remeasured |
| clippy errors | 694 | 0 | 694 not remeasured |
| clippy warnings | 358 | 0 | 358 not remeasured |
| largest module LOC | 18,748 (`scan.rs`) | none >~3k | 18,748 (`scan.rs`) |

### Test dashboard (must never regress)

| Suite | Count |
|---|---|
| `cargo test -p tectonic --test tex-outputs` | 29/29 |
| `cargo test -p tectonic --test trip` | 2/2 |
| `cargo test -p tectonic --test engine_rust` | 7/7 |
| `cargo test -p tectonic --test latex_goldens` | 4/4 |

## Decisions log (Stage 2)

-   **2026-05-23** — End-state target = **full Phase C** (state threaded
    as `&mut EngineState`, near-zero `unsafe`, no internal
    `#[no_mangle]`). Chosen by the user over the thread-local middle
    ground and the pragmatic option.
-   **2026-05-23** — First workstream = **state & memory redesign (01)**,
    as the root cause that dissolves the 688-block clippy bucket and
    sets module boundaries.
-   **2026-05-23** — **Do not** bulk-add SAFETY comments to clear
    `undocumented_unsafe_blocks` before 01/03; let those lints stay red
    in the interim.
-   **2026-05-23** — Type taxonomy correction: `MemIdx` means a valid
    arena index only. Sentinel-carrying TeX halfwords use a distinct
    `MemPtr`/`Halfword` wrapper; `FontId` stays `i32`-backed initially;
    `NodeType` construction is checked, never unchecked `transmute`.
-   **2026-05-23** — Ownership-transfer rule: every state-cluster move
    must update C symbol definitions and `tt_cleanup`/`#[link_name]`
    aliases in the same commit to avoid stale symbols and double frees.
-   **2026-05-23** — `xetex-macos.c` is out of scope for Stage 2. Do not
    translate it or let macOS-only references block Linux-validated
    idiomatization work.
-   **2026-05-23** — `01a-global-inventory.md` created. Workstream 01 is
    READY for execution, starting with least-coupled non-`misc` clusters.
    `misc` rows are a holding area and require final sub-struct decisions
    before migration.
-   **2026-05-23** — Phase-B migrations keep the raw `i32`/`u8`
    representation by design (newtypes are the §4(b) sweep), **but** a
    field whose C type was a semantic typedef/enum (`selector_t`,
    `tt_history_t`, `scaled_t`, …) must get a row in the new
    **Deferred newtype worklist** in `01a-global-inventory.md` in the
    same commit that migrates it. This closes the gap where moving a
    `selector_t` into `EngineState` as `i32` would silently lose the type
    forever. Worklist seeded retroactively for `selector`/`history`/etc.
    Goal is *more* type safety, recovered on a tracked schedule — not
    less.
-   **2026-05-23** — Arena ownership pattern established on `save_stack`
    (session 11): array storage becomes a `Vec<T>` on `EngineState`,
    allocated at init and dropped by `tt_cleanup`; accessors index the
    `Vec`. longjmp/Drop (§7) is handled by keeping the owner in the
    `'static` thread-local, so abort never jumps over live storage and
    `Drop`-on-abort is intentionally skipped (matches prior behavior).
    Reuse this exact shape for the remaining arena arrays.

## Next action for a fresh session

0.  Follow the **Execution recipe** in `01-state-and-memory.md` §11
    (grep checks → add field → repoint accessors → remove defs/aliases →
    worklist rows → dashboard → docs → commit) and heed its recurring
    gotchas.
1.  Continue executing workstream 01. Clean (no-cleanup-alias) scalar
    groups are getting sparse: a few non-hot `scanner` scalars remain
    (`shown_mode`, `mag_set`, `radix`, `expand_depth_count`, …). The
    `mem` bookkeeping scalars (`var_used`, `dyn_used`, `lo_mem_max`,
    `hi_mem_min`, `avail`, `rover`, `mem_end`) are best done as part of
    the `Mem` arena move (§5), not as a loose scalar slice. **Caution:**
    do not bridge the hot tokenizer scalars
    (`cur_cmd`, `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) through the
    thread-local without benchmarking — they are read on nearly every
    `get_next`/`get_token`, and a `RefCell` borrow per access could
    regress performance. Prefer threading those in Phase C.
    The arena-ownership pattern is now proven on `save_stack`
    (session 11), `param_stack` (session 12, first `input`-cluster
    array), and the five `*mut i32` line/group stacks (session 13).
    Ownership-bearing options remain, in rough difficulty order:
    (a) the **rest of the `input` cluster arrays** — `*mut bool`
    `eof_seen` is the last simple one (`vec![false; max_in_open + 1]`,
    same no-dump property, exact session-13 shape, plain `bool` so no
    newtype row); then the trickier element types `input_file`
    (`*mut *mut c_void`), `input_stack` (`*mut InputState`), `nest`
    (`*mut ListStateRecord`), and `buffer` (`*mut i32`). **Gotcha:** each
    input array has **two** alias decls (alloc `tre_*` + cleanup
    `tt_cleanup_*`) to remove, not one;
    (b) `strings` (`str_start` / `str_pool`), which **are** serialized in
    format dump/undump, so that slice must also reconcile the load/dump
    paths; (c) remaining `shipout` file-handle state (`rust_stdout` has a
    cleanup alias; `write_file`/`write_open` are inline arrays movable
    like scalars; `current_diagnostic` is C-defined in `xetex-output.c`).
    For each symbol moved, use `01a-global-inventory.md` for C defs /
    cleanup aliases / accessors, and add a worklist row for any
    semantic-typed field (§4 rule).
2.  The `config` bridge pattern in `src/state.rs` is intentionally small:
    keep `with_state` / `with_state_mut` borrows to single-field
    read/write closures and avoid calling translated engine code while a
    borrow is live.
3.  Do not migrate `misc` inventory rows until their final sub-structs
    are decided.
4.  Cheap clippy wins (04) are always available as low-risk session
    filler, but do not touch the unsafe-block bucket yet.
