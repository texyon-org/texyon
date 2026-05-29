# Idiom session log

Append-only log for Stage 2 (idiomatization). Newest entry at the
**top**, under this header. Distinct from the Stage 1 `../SESSION_LOG.md`
(translation), which is closed.

Each session: what it did (plan construction or execution), the test
dashboard if code changed, surprises, and a handoff.

---

## Idiom session 44 — 2026-05-29 — `misc` sub-struct planning (no code change)

**Type:** planning (workstream 01). Per `STATUS.md` → "Next action" step 1,
session 44 was designated a **sub-struct planning session, not a migration**.
No Rust/C code changed; only `plan/idiom/*` edited. This overrides the generic
"do a migration" framing in the session kickoff — the plan docs are the source
of truth.

**Goal:** resolve the former `misc` holding bucket into concrete `EngineState`
sub-structs so sessions 45+ can batch-migrate instead of moving 1–3 globals at a
time. Update `01a-global-inventory.md` cluster columns + decision block; fix
STATUS's stale Next-action prose.

**Verification done before deciding:**
- The `misc` bucket = 34 raw extern globals + 8 private module-local statics
  (matches the inventory summary counts 34 + 8).
- The 34 raw externs are all line-breaking / LR-reverse / hyphenation-attempt
  scratch: `grep` shows they are used in `src/linebreak.rs`
  (`last_leftmost_char`/`last_rightmost_char` also in `src/scan.rs` via the
  LR-reverse helper). Cold relative to the tokenizer (run per paragraph), so
  safe to bridge through the thread-local in Phase B.
- None of the 34 have a `#[link_name]` alias in `src/ini.rs` (grep: none) and
  none are in the format dump → pure BSS scalar/inline-array move (sessions
  39–43 shape), **not** an ownership slice.
- The fields STATUS's earlier prose floated as `MathState`/`DviState`/
  `OutputState`/`NativeTextState` candidates (`cur_mlist`, `cur_style`,
  `total_pages`, `cur_box`, `native_text`, `space_class`, `LR_ptr`, `cur_dir`,
  …) are **already classified `scanner`/`shipout`** in the inventory — verified
  by grep against the inventory rows. They need no new sub-struct, only
  eventual migration into their assigned clusters. **No `MathState`/`DviState`
  created.**
- `EngineState` is stored *inline* in the thread-local `RefCell`
  (`src/state.rs:502`), not boxed → a 2 KB inline array (`hlist_stack[513]`)
  risks the session-26 `engine_rust` thread-stack overflow during
  `EngineState::default()`. Decision: heap-back `hlist_stack` as `Vec<i32>`.

**Decisions (written into `01a-global-inventory.md`):**
- **`LinebreakState`** (new sub-struct) ← the 34 former-`misc` raw externs.
  `hlist_stack` heap-backed `Vec<i32>` (513); small arrays stay inline.
  Target: session 45.
- **`ShipoutState`** (extend) ← 4 pagebuilder private statics
  (`BEST_PAGE_BREAK`, `BEST_SIZE`, `LEAST_PAGE_COST`, `PAGE_MAX_DEPTH`).
  Target: session 46.
- **`EngineState.synctex: SynctexCtxt`** (new field) ← `SYNCTEX_CTXT`.
  Target: session 46.
- **`TexmfState`** (new sub-struct) ← `LAST_SOURCE_NAME` (owned C string —
  note free lifecycle), `LAST_LINENO`, `START_TIME_STR[30]`. Target: session 46.

**Newtype debts:** none added (no fields migrated). Candidate `scaled_t`/
`MemPtr`/`small_number` rows for the linebreak fields are pre-listed in the
decision block; the actual worklist rows get added at session 45 per the §4
rule (in the migrating commit).

**Metrics:** unchanged (no code touched). Raw extern globals still 139.

**Validation:** none required — no code changed. Did not run the dashboard
(planning-only session; nothing to regress).

**For the next session (45):**
- Execute the `LinebreakState` batch migration per §11. The grep checks are
  pre-done (see decision block) but re-confirm before deleting C defs.
- C defs to remove span **two** files: 6 fields in `xetex/xetex-ini.c`
  (`last_leftmost_char`, `last_rightmost_char`, `hlist_stack`,
  `hlist_stack_level`, `first_p`, `global_prev_p`) and 28 in
  `xetex/xetex-linebreak.c` (the rest) — grep each, watch for symbols
  interleaved with kept ones (e.g. `no_shrink_error_yet`, `hyf_*` sit between
  the linebreak rows and belong to `errors`/`hyph`).
- `LinebreakState` needs a hand-written `impl Default` (contains `Vec` +
  fixed arrays; `hlist_stack` Vec starts empty and is sized at init — confirm
  whether init writes it or it's zero-filled; C BSS is zero, so `vec![0;513]`
  at init or lazy-size both work, match the C alloc shape).
- Add §4 newtype-worklist rows in the same commit.

---

## Idiom session 43 — 2026-05-29 — read_file[16] → InputClusterState

**Type:** execution (workstream 01, Phase-B; inline BSS array slice).

**Goal:** move `read_file[16]` (`UFILE *[16]`) from a C BSS array into
`EngineState.input.read_file: [*mut libc::c_void; 16]`. One coherent slice, one commit.

**Pre-move grep checks (§11 step 2):**
- (a) C type: `UFILE *[16]` (`xetex-xetexd.h:535`). Plain opaque pointer array, no semantic
  typedef.
- (b) cleanup aliases: none — BSS global, no `#[link_name]` aliases in `src/ini.rs`.
- (c) C readers: `xetex-ini.c` only (grep of `xetex/*.c` confirmed). No `.cpp` hits.
- (d) cross-module extern redeclarations: none found in `src/*.rs`.

**Work done (1 commit):**
- Added `read_file: [*mut libc::c_void; 16]` to `InputClusterState` in `src/state.rs`.
  Covered by `#[derive(Default)]` (raw pointers default to `null_mut()`; array of 16 defaults
  to all-null, matching C BSS zero-init).
- `read_file_entry(n)` repointed to `with_state(|s| s.input.read_file[n]) as *mut UFile`
  (no `unsafe` needed for the closure itself).
- `read_file_ptr(n)` repointed to `with_state_mut(|s| unsafe { s.input.read_file.as_mut_ptr().add(n) ... })`
  — same pointer-escaping shape as `input_file_ptr` (session 16): valid past the borrow because
  the array lives in the `'static` thread-local and is never moved or reallocated.
- Removed `pub(super) static mut read_file` from `mod raw` in `src/globals.rs`.
- Removed C definition from `xetex/xetex-ini.c`.

**Newtype debt (§4 check):**
- `read_file`: plain opaque `UFILE*`, stored as `*mut c_void`. No semantic typedef. No debt.
  Confirmed.

**Metrics after this session:**
- `globals.rs` raw extern globals: 140 → 139 (−1).
- Raw `unsafe { ... }` text-search count: 367 → 366 (−1).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- Leftover C references: clean for `read_file` (grep confirmed empty).
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`:
  pass (29/2/7/4).

**For the next session:**
- All simple BSS inline-array candidates in the `input` cluster are exhausted.
- `xdv_buffer` (`*mut c_char`) and `mapped_text` (`*mut u16`) are the next reachable candidates
  but require rewriting the `ext.rs` alloc/use pattern (more involved than scalar moves).
- `sa_root` (has link alias → ownership slice), `sa_null` (`MemoryWord`, no Default),
  `max_reg_help_line` (pointer) remain in `scanner`.
- `name_of_file` / `name_of_file16` remain deferred (raw pointer buffers, ownership not decided).

---

## Idiom session 42 — 2026-05-29 — name_length / name_length16 / read_open → InputClusterState

**Type:** execution (workstream 01, Phase-B; scalar + inline-array slice).

**Goal:** move `name_length` (`int32_t`), `name_length16` (`int32_t`), and `read_open[17]`
(`unsigned char[17]`) from C BSS globals into `EngineState.input` (`InputClusterState`). One
coherent slice, one commit.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `name_length` and `name_length16` are `int32_t` (`xetex-xetexd.h:390–391`).
  `read_open` is `unsigned char[17]` (`xetex-xetexd.h:536`). No semantic typedefs.
- (b) cleanup aliases: none — all three are BSS globals, no `#[link_name]` aliases in `src/ini.rs`.
- (c) C readers: `xetex-ini.c` only for all three (confirmed via grep of `xetex/*.c`; no `.cpp`
  hits). Clean.
- (d) cross-module extern redeclarations: none found in `src/*.rs`.

**Work done (1 commit):**
- Added `name_length: i32`, `name_length16: i32`, and `read_open: [u8; 17]` to
  `InputClusterState` in `src/state.rs`. All three are covered by `#[derive(Default)]` (zero
  default for scalars; `[0u8; 17]` for the array — matches C BSS zero-init). The engine
  explicitly initializes `read_open` entries to `CLOSED` (= 2) during init, so the default value
  is an unobservable intermediate state.
- Repointed 6 accessor bodies in `src/globals.rs` from `unsafe { raw::SYM }` to
  `with_state`/`with_state_mut` closures.
- Removed 3 `pub(super) static mut` declarations from `mod raw` in `src/globals.rs`.
- Removed 3 C definitions from `xetex/xetex-ini.c` (lines 24–25 and 85 in the original).

**Newtype debt (§4 check):**
- `name_length`, `name_length16`: plain `int32_t`, no semantic typedef, no debt.
- `read_open`: `unsigned char`, no semantic typedef, no debt. Confirmed.

**Metrics after this session:**
- `globals.rs` raw extern globals: 143 → 140 (−3).
- Raw `unsafe { ... }` text-search count: 373 → 367 (−6).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- Leftover C references: clean for all 3 symbols (grep confirmed empty).
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`:
  pass (29/2/7/4).

**For the next session:**
- `read_file[16]` (inline BSS `*mut c_void[16]`) is the next clean inline-array candidate; it
  has a pointer-escaping accessor (`read_file_ptr`) — same shape as `buffer_ptr`/`input_file_ptr`,
  so should be compatible.
- `xdv_buffer` / `mapped_text` require ext.rs alloc/use rewrite (more involved).
- `sa_root`, `sa_null`, `max_reg_help_line` remain for their ownership/non-Default reasons.
- Hot tokenizer scalars (`cur_cmd`, `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) remain deferred.

---

## Idiom session 41 — 2026-05-29 — font_info arena + 30 scalar/array globals → state sub-structs

**Type:** execution (workstream 01, Phase-B; six ownership/scalar slices in one large session).

**Goal (six commits):**
1. `font_info` (`memory_word *`, the main font metric arena) → `Vec<MemoryWord>` on `EngineState.fonts`.
2. 12 input cluster scalars → `InputClusterState` scalar fields: `open_parens`, `in_open`,
   `pseudo_files`, `nest_ptr`, `max_nest_stack`, `input_ptr`, `max_in_stack`, `base_ptr`,
   `par_loc`, `par_token`, `param_ptr`, `max_param_stack`.
3. 5 conditionals scalars → `ScannerState`: `cond_ptr`, `if_limit`, `cur_if`, `if_line`, `skip_line`.
4. 7 misc scanner/input scalars → `ScannerState` / `InputClusterState`: `is_in_csname`,
   `cur_val1`, `cur_val_level`, `used_tectonic_coda_tokens` (scanner); `force_eof`, `last`,
   `max_buf_stack` (input).
5. 5 scanner-status scalars → `ScannerState`: `scanner_status`, `warning_index`, `def_ref`,
   `long_state`, `align_state`.
6. 2 inline fixed arrays → `ScannerState`: `cur_mark[5]`, `disc_ptr[4]`.

**Pre-move grep checks (§11 step 2, all six slices):**
- (a) C types: `font_info` is `memory_word *` (no semantic typedef). All scalars are `int32_t`
  or `unsigned char` or `bool`; inline arrays are `int32_t[N]`. No semantic typedef except
  `cur_if` (C type `small_number` = `typedef short`).
- (b) cleanup aliases: `font_info` had alloc alias `src/ini.rs:1822` + cleanup alias `src/ini.rs:3669`.
  All other globals (scalars + inline arrays) had zero link aliases.
- (c) C readers: all symbols only in `xetex-ini.c`. `last`/`max_buf_stack` also nominally
  local variable–named in `teckit-Engine.cpp` (confirmed local, not the C global).
- (c, extended) C++ readers: none found for any symbol.
- (d) cross-module redeclarations: none found.

**Work done (6 commits, 1 per slice):**
- Commit 1: Added `font_info: Vec<MemoryWord>` to `FontState`; repointed 4 accessors; replaced
  initex alloc (`xmalloc_array`) and format-load alloc/undump with `vec![zero_word; fms+1]` /
  `undump_slice` inside `with_state_mut`; replaced dump with `with_state` closure; replaced both
  cleanup `libc::free` calls with `state.fonts.font_info = Vec::new()`; removed 2 link aliases,
  1 raw extern, 1 C definition. Pre-computed `fmem_ptr()`/`font_mem_size()` before every borrow
  (session-39 nested-borrow gotcha).
- Commits 2–6: Added scalar/array fields to `InputClusterState` / `ScannerState`; removed raw
  statics from `mod raw`; repointed all getters/setters from `unsafe { raw::SYM }` to
  `with_state` / `with_state_mut` closures; removed all C definitions from `xetex-ini.c`.

**New gotcha (session 41):** Writing to a union field via `&mut` (`MemoryWord.b32.s1 = v`) through
a Vec index is **safe** in Rust (union *write* via mutable reference needs no `unsafe`). The
compiler's `unused_unsafe` warning confirms this. Reading a union field still requires `unsafe`.
Updated the new accessors accordingly. This is the opposite of what session 39 assumed for direct
C-pointer writes; it's a consequence of moving from `*mut MemoryWord` to `Vec<MemoryWord>`.

**Newtype debt (§4 check):**
- `font_info`: plain `memory_word *`, no debt.
- `cur_if`: C type `small_number` (typedef for `short`, stored `i16`) — worklist row added to
  01a-global-inventory.md (same category as `radix`, session 21).
- All other 30 fields: plain `int32_t`/`bool`/`unsigned char`/`int32_t[N]`, no debt. Confirmed.

**longjmp / Drop (§7):** all Vecs live in the `'static` thread-local; Drop on abort intentionally skipped.

**Metrics after this session:**
- `globals.rs` raw extern globals: 175 → 143 (−32 this session; was 175 at session start).
- Raw `unsafe { ... }` text-search count: 437 → 373 (−64 this session).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass after each commit, pre-existing warnings only.
- Leftover C references: clean for all 32 symbols (grep confirmed empty).
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`:
  pass (29/2/7/4) verified after commits 1, 2, and 6.

**For the next session:**
- `fonts` cluster: `xdv_buffer` (`*mut c_char`) and `mapped_text` (`*mut u16`) remain. Both are
  dynamically-managed write buffers used by `ext.rs` itself (free + realloc within Rust); `ext.rs`
  has its own `extern "C"` block for them. Conversion to `Vec<u8>` / `Vec<u16>` is feasible but
  requires rewriting the ext.rs allocation/use pattern (similar to `buffer_ptr` session-15 shape).
- Clean cold scalar filler is getting sparse. Good next candidates:
  - `read_open[17]` (inline `u8[17]`, BSS, no alloc edge — scalar-style move, confirmed clean).
  - `read_file[16]` (inline `*mut c_void[16]`, but used by `read_file_ptr` pointer-escaping accessor — check first).
  - `name_length` / `name_length16` (`int32_t`, only in xetex-ini.c, no aliases — clean scalars).
  - `name_of_file` / `name_of_file16` (raw pointer buffers — more complex, defer until ownership pattern for these buffers is decided).
- Hot tokenizer scalars (`cur_cmd`, `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) remain deferred to Phase C.
- `mem` bookkeeping scalars (`var_used`, `dyn_used`, `lo_mem_max`, etc.) remain deferred to the `Mem` arena slice.
- Keep applying the session-39/41 pre-borrow rule: any `with_state`-backed value must be
  pre-computed before entering a `with_state_mut` closure.

---

## Idiom session 40 — 2026-05-29 — hyphen_char + skew_char → FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move `hyphen_char` and `skew_char` (both `int32_t *`, format-serialized) from C `malloc`
storage into Rust-owned `Vec<i32>` fields on `EngineState.fonts`. One coherent slice, one commit.

**Pre-move grep checks (§11 step 2):**
- (a) C types: both `int32_t *` in `xetex-xetexd.h:568–569`. Plain type, no semantic typedef.
- (b) cleanup aliases: 4 total — alloc aliases at `src/ini.rs:1829/1831`, cleanup aliases at
  `src/ini.rs:3671/3673`. Ownership slice confirmed.
- (c) C readers: only `xetex-ini.c` (definition). No other `.c` files.
- (c, extended) C/C++ readers: grep of `xetex/*.cpp` returned empty. No bridges needed.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs` link aliases; no
  unexpected cross-module redeclarations.

**Work done (1 commit):**
- Added `hyphen_char: Vec<i32>` and `skew_char: Vec<i32>` fields to `FontState` in `src/state.rs`;
  added `Vec::new()` defaults to the manual `Default` impl.
- Repointed 4 accessors in `src/globals.rs` from raw pointer offset to `with_state` /
  `with_state_mut` Vec indexing; removed 2 `pub(super) static mut` raw statics.
- Replaced `xmalloc_array::<i32>(font_max())` initex allocs with `vec![0i32; font_max()+1]` folded
  into the existing `with_state_mut` block alongside the `*_base` arrays.
- Replaced raw-pointer `dump_slice` calls with a `with_state` closure over the Vec storage.
- Replaced raw-pointer `undump_slice` calls with a `with_state_mut` closure over the Vec storage.
- Replaced `xcalloc_array::<i32>(font_max())` format-load allocs with `vec![0i32; font_max()+1]`
  folded into the existing `with_state_mut` block alongside the `*_base` arrays.
- Replaced `libc::free(tt_cleanup_hyphen_char)` and `libc::free(tt_cleanup_skew_char)` cleanup
  calls with `Vec::new()` resets inside the existing `with_state_mut` cleanup block.
- Removed both alloc link aliases (`tre_hyphen_char`/`tre_skew_char`), both cleanup link aliases
  (`tt_cleanup_hyphen_char`/`tt_cleanup_skew_char`), 2 raw externs, and 2 C definitions from
  `xetex-ini.c`.

**Newtype debt (§4 check):** both are plain `int32_t *` — no semantic typedef, no worklist row
needed. Checked and confirmed.

**longjmp / Drop (§7):** both Vecs live in the `'static` thread-local `EngineState`; Drop on abort
intentionally skipped.

**No surprises:** the `with_state` nested-borrow gotcha from session 39 did not apply here; all
state accessor calls (`font_max()`, `font_ptr()`) were already pre-computed outside the
`with_state_mut` closures (session-39 pattern was already in place).

**Metrics after this session:**
- `globals.rs` raw extern globals: 177 → 175 (−2).
- Raw `unsafe { ... }` text-search count: 441 → 437 (−4, accessor body cleanup).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `grep` of `hyphen_char`/`skew_char` in `xetex/*.c` and `xetex/*.cpp` (minus macos): empty.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`:
  pass (29/2/7/4).

**For the next session:**
- The `fonts` cluster remaining C externs: `font_info` (`*mut MemoryWord`, the main font metric
  arena — large ownership slice), `xdv_buffer` (`*mut c_char`), `mapped_text` (`*mut u16`).
- `hyphen_char` and `skew_char` are now Rust-owned. The only remaining simple format-serialized
  font arrays are done; next is either the large `font_info` arena or a different cluster.
- Good next candidate: `font_info` arena (ownership shape: `Vec<MemoryWord>`, large; establishes
  the §5 Mem arena pattern for the main memory). Alternatively move to an unrelated cluster
  (e.g., `eqtb` ownership slice or a `misc` scalar group whose sub-struct is decided).
- Continue applying the §11 nested-borrow rule.

---

## Idiom session 39 — 2026-05-29 — fonts *_base arrays, flags/letter-space/fmem/font-ptr, mapping/engine → FontState

**Type:** execution (workstream 01, Phase-B; three ownership/scalar slices in one session).

**Goal:** move the remaining per-font metadata arrays and scalars into `EngineState.fonts`:
1. 9 `*_base` offset arrays (`char_base`, `width_base`, `height_base`, `depth_base`, `italic_base`,
   `lig_kern_base`, `kern_base`, `exten_base`, `param_base`) — all format-serialized `Vec<i32>`.
2. `font_flags` (`char *` → `Vec<i8>`), `font_letter_space` (`scaled_t *` → `Vec<i32>`),
   `fmem_ptr` (`font_index`), `font_ptr` (`internal_font_number`) — not format-serialized; `fmem_ptr`
   and `font_ptr` are plain scalars without cleanup aliases.
3. `font_mapping` and `font_layout_engine` (both `void **` → `Vec<*mut c_void>`) — not format-
   serialized; handles released explicitly in `tt_cleanup` before Vec drop; `font_layout_engine` had
   19 direct C++ reads requiring a new bridge.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `*_base` arrays are plain `int32_t *`; `font_flags` is `char *`; `font_letter_space`
  is `scaled_t *`; `fmem_ptr` is `font_index`; `font_ptr` is `internal_font_number`; both pointer
  arrays are `void **`.
- (b) cleanup aliases: all 9 `*_base` arrays had init+cleanup aliases (18 total); `font_flags` and
  `font_letter_space` had init+cleanup aliases (4 total); `font_mapping`/`font_layout_engine` had
  init+cleanup aliases (4 total); `fmem_ptr`/`font_ptr` had none.
- (c) C readers: only `xetex-ini.c` for all 15 symbols.
- (c, extended) C/C++ readers: `font_layout_engine` was directly read in `xetex-XeTeXOTMath.cpp`
  (19 uses); all others were clean.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs` aliases; none elsewhere.

**Work done (3 commits):**

*Commit 1 (9 `*_base` arrays):*
- Added 9 `Vec<i32>` fields to `FontState`.
- Repointed 18 accessors (9 getter/setter pairs) to index state Vecs.
- Replaced initex and format-load allocs with `vec![0i32; font_max+1]` inside `with_state_mut`.
- Consolidated format dump into a single `with_state` closure; consolidated undump into a single
  `with_state_mut` closure (pre-computed `base_undump_count = font_ptr() + 1` outside the borrow).
- Dropped all 9 in `tt_cleanup` via a single `with_state_mut` reset block.
- Removed 9 raw externs, 18 `src/ini.rs` link aliases, and 9 C definitions.

*Commit 2 (`font_flags`, `font_letter_space`, `fmem_ptr`, `font_ptr`):*
- Added 4 fields to `FontState` (`Vec<i8>`, `Vec<i32>`, `i32`, `i32`).
- Repointed 6 accessors.
- Replaced allocs; `fmem_ptr`/`font_ptr` are plain scalar fields with no alloc/cleanup.
- Folded `font_flags`/`font_letter_space` resets into the existing `with_state_mut` cleanup block.
- Removed 4 raw externs, 4 link aliases, 4 C definitions.
- **Gotcha (new):** `fmem_ptr()` and `font_ptr()` now go through `with_state` (RefCell borrow).
  Any `with_state_mut` closure that contains `fmem_ptr()` or `font_ptr()` will panic with
  "RefCell already mutably borrowed". Fixed by precomputing the values *before* each
  `with_state_mut` closure that needs them (e.g., `let font_undump_count = font_ptr() + 1;`
  before the undump closures). Three existing format-load undump closures had this bug.

*Commit 3 (`font_mapping`, `font_layout_engine`):*
- Added 2 `Vec<*mut c_void>` fields to `FontState` (null-initialized).
- Repointed 4 accessors; added `#[no_mangle] tt_get_font_layout_engine_entry` Rust bridge.
- Replaced `xcalloc_array` allocs with `vec![null_mut(); font_max+1]` inside `with_state_mut`;
  removed the now-redundant loop that null-initialized the C mapping array.
- Replaced `libc::free` cleanup calls with Vec resets in `with_state_mut`.
- Removed 2 raw externs, 4 link aliases, 2 C definitions.
- Replaced all 19 `font_layout_engine[f]` uses in `xetex-XeTeXOTMath.cpp` with
  `tt_get_font_layout_engine_entry(f)`.

**Newtype debt (§4 check):** rows added for `fmem_ptr` (`font_index`), `font_ptr`
(`internal_font_number`), and `font_letter_space[]` (`scaled_t *`). The 9 `*_base` arrays and
`font_flags`/`font_mapping`/`font_layout_engine` are plain integer/char/pointer types — no rows.

**longjmp / Drop (§7):** all Vecs live in the `'static` thread-local `EngineState`; handles are
released explicitly before Vec drop; longjmp/abort paths skip Drop as intended.

**Gotcha (new, worth adding to §11 recurring gotchas):** once a scalar is backed by `with_state`,
its getter must NOT be called inside a `with_state_mut` closure — RefCell panics with "already
mutably borrowed". Always pre-compute the value outside the mutable borrow scope.

**Metrics after this session:**
- `globals.rs` raw extern globals: 192 → 177 (−15 across 3 commits).
- Raw `unsafe { ... }` text-search count: 471 → 441 (−30).
- `#[no_mangle]` exports: 578 → 579 (+1, `tt_get_font_layout_engine_entry` bridge).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- First dashboard run after Commit 2 failed with "RefCell already mutably borrowed" in `align_span`
  (engine_rust test); root cause: `font_ptr() + 1` inside 3 `with_state_mut` undump closures.
  Fixed by pre-computing `let font_undump_count = font_ptr() + 1` before those closures.
- Final `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4).
- `grep` of all 15 moved symbols in `xetex/*.{c,cpp}` (minus macos): empty of C definitions.

**For the next session:**
- The `fonts` cluster is now nearly fully Rust-owned. Remaining C externs:
  - `font_info` (`*mut MemoryWord`) — the main font metric arena; large ownership slice.
  - `hyphen_char`, `skew_char` (still C malloc'd `*mut i32` arrays; have init+cleanup aliases).
  - `xdv_buffer` (`*mut c_char`) — XDV output buffer.
  - `mapped_text` (`*mut u16`) — TECkit output scratch buffer.
  - A few session-35 stale inventory rows: `font_name`, `font_area`, `font_params`, etc. are
    already Rust-owned; their inventory rows need updating.
- Pick a coherent bounded next subgroup:
  - `hyphen_char` + `skew_char` are an obvious pair (same shape as prior `*_base` arrays, no C++
    readers expected, both format-serialized).
  - Alternatively, move to the `eqtb` or `input` clusters, or a misc scalar group.
- Continue applying the §11 nested-borrow rule: always pre-compute state-accessor values before
  any `with_state_mut` closure that needs them.

---

## Idiom session 38 — 2026-05-29 — font_glue + font_used arrays -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the two remaining simple per-font table arrays — `font_glue`
(`int32_t *`, format-dumped) and `font_used` (`bool *`, not format-dumped) —
out of C `malloc` storage and into Rust-owned state on `EngineState.fonts`.
One coherent slice, one commit.

**Why this slice:** STATUS / session-37 handoff named `font_glue` and
`font_used` as the next candidates. They are the two next per-font table
arrays, share the established `FontState` ownership shape, and are not hot
tokenizer/main-loop scratch state.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `extern int32_t *font_glue;` and `extern bool *font_used;`
  (xetex-xetexd.h:566-567).
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation aliases
  (`tre_font_glue`, `tre_font_used`) and cleanup aliases
  (`tt_cleanup_font_glue`, `tt_cleanup_font_used`); all four were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (c, extended) C/C++ readers: only `xetex/xetex-ini.c` (`.cpp` grep clean).
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other Rust redeclarations.

**Work done:**
- Added `font_glue: Vec<i32>` and `font_used: Vec<bool>` to `FontState`.
- Repointed `font_glue`/`set_font_glue` and
  `font_used_entry`/`set_font_used_entry` to index the Rust-owned vectors.
- Folded `font_glue` into both the initex and format-load init closures
  (`vec![0i32; font_check_len]`), replacing the `xmalloc_array`/`xcalloc_array`
  calls. `font_used` is allocated unconditionally as
  `vec![false; font_max + 1]` (matching the former xmalloc + zero loop).
- Repointed format dump to a state-derived `font_glue` pointer at `FONT_BASE`.
- Repointed the checked undump (`undump_checked_i32` against
  `MIN_HALFWORD..lo_mem_max`) to state-derived storage, precomputing the count
  (`font_ptr() + 1`) and bound (`lo_mem_max()`) before entering
  `with_state_mut` to avoid nested borrows. `font_used` is not in the format
  dump.
- Dropped both vectors in `tt_cleanup`.
- Removed all four `#[link_name]` aliases, the two raw externs in
  `globals.rs`, and the two C definitions in `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** `font_glue[]` is declared `int32_t *` but holds a
nullable glue-spec memory pointer (checked on undump against `lo_mem_max`), so
a worklist row (`Vec<MemPtr>`) was added. `font_used[]` is plain `bool *` — no
row needed (checked).

**longjmp / Drop (§7):** owned vectors live in the `'static` thread-local
`EngineState` and are released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** none. Extended `.cpp` reader grep was clean.

**Metrics after this session:**
- `globals.rs` raw extern globals: 194 -> 192 (-2).
- Raw `unsafe { ... }` text-search count: 475 -> 471 (-4, four getter/setter
  bodies became fully safe).
- `#[no_mangle]` exports: unchanged at 578.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_glue`/`font_used` in `xetex/*.{c,cpp}` (minus macos): empty.

**For the next session:**
- The simple per-font table arrays are now exhausted. Remaining `fonts` state
  is messier: pointer-element arrays (`font_mapping`, `font_layout_engine`),
  the `font_info` arena and its `*_base` index tables, and various scalars.
  Pick a coherent bounded subgroup and keep including `.cpp` in reader greps.
- Alternatively pick an ownership-aware `eqtb`/`mem` slice (but do not split
  the `mem` bookkeeping scalars from the `Mem` arena move).

## Idiom session 37 — 2026-05-29 — font boundary-character arrays -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the coherent boundary-character font subgroup
(`bchar_label`, `font_bchar`, and `font_false_bchar`) out of C `malloc`
storage and into Rust-owned state on `EngineState.fonts`. One larger but
bounded ownership slice, one commit.

**Why this slice:** the user asked for a bigger session, and these three arrays
are adjacent per-font boundary/ligature metadata with the same
init/dump/cleanup shape. They are not hot tokenizer/main-loop scratch state,
and their reader greps are clean.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `bchar_label` is `font_index *`; `font_bchar` and
  `font_false_bchar` are `nine_bits *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name]` aliases for all three symbols; all six were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (c, extended) C/C++ readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other Rust redeclarations.

**Work done:**
- Added `bchar_label: Vec<i32>`, `font_bchar: Vec<i32>`, and
  `font_false_bchar: Vec<i32>` to `FontState`.
- Repointed the three getter/setter pairs to index the Rust-owned vectors.
- Replaced init and format-load allocation with `vec![0i32; font_max + 1]`
  for all three arrays.
- Repointed format dump and checked undump to state-derived storage at
  `FONT_BASE`; computed the repeated `font_count`/`fmem_ptr()` bounds before
  entering the mutable state closures.
- Dropped all three vectors in `tt_cleanup`.
- Removed all six `src/ini.rs` `#[link_name]` aliases, the raw externs in
  `globals.rs`, and the C definitions in `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** added rows for `bchar_label[]`
(`font_index *` -> `Vec<FontIndex>`) and `font_bchar[]` /
`font_false_bchar[]` (`nine_bits *` -> `Vec<NineBits>`) to the deferred
newtype worklist.

**longjmp / Drop (§7):** owned vectors live in the `'static` thread-local
`EngineState` and are released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** none. Extended `.cpp` reader grep was clean.

**Metrics after this session:**
- `globals.rs` raw extern globals: 197 -> 194 (-3).
- Raw `unsafe { ... }` text-search count: 481 -> 475 (-6, three
  getter/setter pairs became fully safe).
- `#[no_mangle]` exports: unchanged at 578.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `bchar_label`/`font_bchar`/`font_false_bchar` in `xetex/*.c`
  (minus macos): empty.
- Extended grep of those symbols in `xetex/*.{c,cpp}` (minus macos): empty.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Good candidates:
  `font_glue` alone (semantic memory pointer / checked undump bound against
  `lo_mem_max`) or `font_used` alone (plain `bool *`, not format-dumped).
- Keep including `.cpp` in reader greps for font-table slices.
- Avoid nested `with_state_mut` borrows in format load paths when bounds
  depend on other state-backed accessors.

## Idiom session 36 — 2026-05-29 — font character-range arrays -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the paired `fonts` character-range tables (`font_bc` and
`font_ec`) out of C `malloc` storage and into Rust-owned state on
`EngineState.fonts`. One coherent ownership slice, one commit.

**Why this slice:** session 35 and `STATUS.md` handed off `font_bc`/`font_ec`
as the next bounded pair. Both are `UTF16_code *`, share the same
init/dump/cleanup shape, and are format-serialized together.

**Pre-move grep checks (§11 step 2):**
- (a) C type: both symbols are `UTF16_code *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name]` aliases for both symbols; all four were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (c, extended) C/C++ readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other Rust redeclarations.

**Work done:**
- Added `font_bc: Vec<u16>` and `font_ec: Vec<u16>` to `FontState`.
- Repointed `font_bc_entry`/`set_font_bc_entry` and
  `font_ec_entry`/`set_font_ec_entry` to index the Rust-owned vectors.
- Replaced init and format-load allocation with `vec![0u16; font_max + 1]`
  for both arrays.
- Repointed format dump and undump to state-derived storage at `FONT_BASE`.
- Dropped both vectors in `tt_cleanup`.
- Removed both `src/ini.rs` `#[link_name]` aliases per array, the raw externs
  in `globals.rs`, and the C definitions in `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** added rows for `font_bc[]` and `font_ec[]`
(`UTF16_code *` -> `Vec<Utf16Code>`) to the deferred newtype worklist.

**longjmp / Drop (§7):** owned vectors live in the `'static` thread-local
`EngineState` and are released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** none. Unlike the previous font slices, the extended `.cpp`
reader grep was clean.

**Metrics after this session:**
- `globals.rs` raw extern globals: 199 -> 197 (-2).
- Raw `unsafe { ... }` text-search count: 485 -> 481 (-4, two getter/setter
  pairs became fully safe).
- `#[no_mangle]` exports: unchanged at 578.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_bc`/`font_ec` in `xetex/*.c` (minus macos): empty.
- Extended grep of `font_bc`/`font_ec` in `xetex/*.{c,cpp}` (minus macos):
  empty.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Likely candidates are
  `font_glue` or `font_used` (both still have `src/ini.rs` allocation/cleanup
  aliases); use `01a-global-inventory.md` and the §11 greps to decide whether
  either should be alone or paired with a closely related table.
- Keep including `.cpp` in reader greps for font-table slices.
- Avoid nested `with_state_mut` borrows in format load paths when bounds
  depend on other state-backed accessors.

## Idiom session 35 — 2026-05-29 — font name/area arrays -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the paired `fonts` string-index tables (`font_name` and
`font_area`) out of C `malloc` storage and into Rust-owned state on
`EngineState.fonts`. One coherent ownership slice, one commit.

**Why this slice:** session 34 handed off another bounded font table subgroup,
and `STATUS.md` called out `font_name`/`font_area` as a good next pair. Both
are `str_number *`, share the same init/dump/cleanup shape, and are
format-serialized together.

**Pre-move grep checks (§11 step 2):**
- (a) C type: both symbols are `str_number *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name]` aliases for both symbols; all four were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (c, extended) C/C++ readers: `xetex/xetex-ini.c` plus a direct
  `font_area` reader in `xetex/xetex-XeTeXOTMath.cpp`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other Rust redeclarations.

**Work done:**
- Added `font_name: Vec<i32>` and `font_area: Vec<i32>` to `FontState`.
- Repointed `font_name_entry`/`set_font_name_entry` and
  `font_area_entry`/`set_font_area_entry` to index the Rust-owned vectors.
- Replaced init and format-load allocation with `vec![0i32; font_max + 1]`
  for both arrays.
- Repointed format dump and checked undump to state-derived storage at
  `FONT_BASE`; fixed the checked undump path to compute `str_ptr()` before
  entering the mutable state borrow.
- Dropped both vectors in `tt_cleanup`.
- Removed both `src/ini.rs` `#[link_name]` aliases per array, the raw externs
  in `globals.rs`, and the C definitions in `xetex/xetex-ini.c`.
- Routed the remaining C++ OpenType math direct `font_area[f]` reads through a
  narrow Rust export, `tt_get_font_area_entry`.

**Newtype debt (§4 check):** added rows for `font_name[]` and `font_area[]`
(`str_number *` -> `Vec<StrNumber>`) to the deferred newtype worklist.

**longjmp / Drop (§7):** owned vectors live in the `'static` thread-local
`EngineState` and are released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** the extended `.cpp` reader grep mattered again:
`xetex-XeTeXOTMath.cpp` read `font_area` directly in OpenType math helpers.
Also, the first dashboard run exposed a nested state borrow in the undump
closure; computing `font_count`/`max_str` outside the closure fixed it.

**Metrics after this session:**
- `globals.rs` raw extern globals: 201 -> 199 (-2).
- Raw `unsafe { ... }` text-search count: 489 -> 485 (-4, two getter/setter
  pairs became fully safe).
- `#[no_mangle]` exports: 577 -> 578 (new C++ bridge).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- First dashboard run failed in `engine_rust` due to the nested `str_ptr()`
  state borrow described above; fixed before final validation.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_name`/`font_area` in `xetex/*.c` (minus macos): empty.
- Extended grep of `font_name`/`font_area` in `xetex/*.{c,cpp}` (minus macos):
  empty of direct array-symbol reads after routing C++ through
  `tt_get_font_area_entry`.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Good next candidate:
  `font_bc`/`font_ec` (`UTF16_code *`, same init/dump/cleanup shape).
- Keep including `.cpp` in reader greps for font-table slices.
- Avoid nested `with_state_mut` borrows when checked undump bounds depend on
  other state-backed accessors; compute lengths/bounds before entering the
  mutation closure.

## Idiom session 34 — 2026-05-23 — font parameter-count array -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the single `fonts` parameter-count table (`font_params`) out of
C `malloc` storage and into Rust-owned state on `EngineState.fonts`. One
bounded ownership slice, one commit.

**Why this slice:** session 33 handed off `font_params` alone as a good next
candidate. It is format-serialized like the previous font tables, but its
`font_index *` type and checked undump path make it cleaner to migrate by
itself instead of mixing it with string or UTF-16 font tables.

**Pre-move grep checks (§11 step 2):**
- (a) C type: `font_params` is `font_index *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name="font_params"]` aliases; both were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (c, extended) C/C++ readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other redeclarations.

**Work done:**
- Added `font_params: Vec<i32>` to `FontState`.
- Repointed `font_params()` and `set_font_params()` to index the Rust-owned
  vector.
- Replaced init and format-load allocation with `vec![0i32; font_max + 1]`.
- Repointed format dump and checked undump to state-derived storage at
  `FONT_BASE`.
- Dropped the vector in `tt_cleanup`.
- Removed both `src/ini.rs` `#[link_name="font_params"]` aliases, the raw
  extern declaration from `globals.rs`, and the C definition from
  `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** added a row for `font_params[]`
(`font_index *` -> `Vec<FontIndex>`) to the deferred newtype worklist.

**longjmp / Drop (§7):** owned vector lives in the `'static` thread-local
`EngineState` and is released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** none. The extra `.cpp` reader grep from session 33 was clean.

**Metrics after this session:**
- `globals.rs` raw extern globals: 202 -> 201 (-1).
- Raw `unsafe { ... }` text-search count: 491 -> 489 (-2, getter/setter
  bodies became fully safe).
- `#[no_mangle]` exports: unchanged at 577.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_params` in `xetex/*.c` (minus macos): empty.
- Extended grep of `font_params` in `xetex/*.{c,cpp}` (minus macos): empty.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Good candidates are
  `font_name`/`font_area` as a pair (`str_number *`, same init/dump/cleanup
  shape) or `font_bc`/`font_ec` as a pair (`UTF16_code *`, same
  init/dump/cleanup shape).
- Keep including `.cpp` in reader greps for font-table slices.
- Avoid nested `with_state_mut` borrows when allocating: compute capacities via
  accessors before entering the state mutation closure.

## Idiom session 33 — 2026-05-23 — font size arrays -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the paired `fonts` size tables (`font_size` and `font_dsize`)
out of C `malloc` storage and into Rust-owned state on `EngineState.fonts`.
One coherent ownership slice, one commit.

**Why this slice:** session 32 handed off another bounded font table subgroup,
with `font_size`/`font_dsize` called out as a good pair. They have the same
`scaled_t *` type, init/dump/cleanup shape, and are semantically paired as the
current at-size and design-size tables.

**Pre-move grep checks (§11 step 2):**
- (a) C type: both symbols are `scaled_t *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name]` aliases for both symbols; all four were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other Rust redeclarations.

**Work done:**
- Added `font_size: Vec<i32>` and `font_dsize: Vec<i32>` to `FontState`.
- Repointed `font_size_entry`/`set_font_size_entry` and
  `font_dsize_entry`/`set_font_dsize_entry` to index the Rust-owned vectors.
- Replaced init and format-load allocation with `vec![0i32; font_max + 1]`
  for both arrays; format dump/undump now uses state-derived pointers at
  `FONT_BASE`.
- Dropped both vectors in `tt_cleanup`.
- Removed both `src/ini.rs` link aliases per array, the raw externs in
  `globals.rs`, and the C definitions in `xetex/xetex-ini.c`.
- Repointed the remaining C++ OpenType math helper direct reads in
  `xetex/xetex-XeTeXOTMath.cpp` through a narrow Rust export,
  `tt_get_font_size_entry`.

**Newtype debt (§4 check):** added rows for `font_size[]` and `font_dsize[]`
(`scaled_t *` -> `Vec<Scaled>`) to the deferred newtype worklist.

**longjmp / Drop (§7):** owned vectors live in the `'static` thread-local
`EngineState` and are released explicitly by `tt_cleanup`; abort paths skip
Drop as intended by the existing arena pattern.

**Surprise:** the documented `.c` reader grep missed
`xetex/xetex-XeTeXOTMath.cpp`, which still read `font_size[f]` directly. The
full dashboard caught this as an undefined symbol at link time. Fixed by adding
the C++ bridge and extending the final local grep to include `.cpp`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 204 -> 202 (-2).
- Raw `unsafe { ... }` text-search count: 495 -> 491 (-4, getter/setter
  bodies became fully safe).
- `#[no_mangle]` exports: 576 -> 577 (+1, temporary C++ bridge for OpenType
  math).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_size` / `font_dsize` in `xetex/*.c` (minus macos): empty.
- Extended grep of `font_size` / `font_dsize` in `xetex/*.{c,cpp}` (minus
  macos): empty.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Good candidates are
  `font_params` alone (`font_index *`, format-serialized, checked undump) or
  the `font_name`/`font_area` pair (`str_number *`, same init/dump/cleanup
  shape). Keep subgroups coherent with init/dump/cleanup edges.
- Include `.cpp` in reader greps for font-table slices; `xetex-XeTeXOTMath.cpp`
  can hold direct table reads that the §11 `.c` grep misses.
- Avoid nested `with_state_mut` borrows when allocating: compute capacities via
  accessors before entering the state mutation closure.

## Idiom session 32 — 2026-05-23 — font checksum array -> FontState

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the single `fonts` checksum table (`font_check`) out of C
`malloc` storage and into Rust-owned state on `EngineState.fonts`. One
coherent ownership slice, one commit.

**Why this slice:** session 31 handed off the ownership-aware font arrays as
the next area. `font_check` is the smallest format-serialized font table with
both init and cleanup aliases, so it exercises the font-array ownership shape
without mixing in the many scalar `i32`/`u16` tables.

**Pre-move grep checks (§11 step 2):**
- (a) C type: `font_check` is `b16x4 *`.
- (b) cleanup/link aliases: `src/ini.rs` had the expected allocation and
  cleanup `#[link_name="font_check"]` aliases; both were removed.
- (c) C readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other redeclarations.

**Work done:**
- Added `font_check: Vec<B16x4>` to `FontState`.
- Repointed `set_font_check()` and `font_check_entry()` to index the Rust-owned
  vector.
- Replaced init and format-load `xmalloc_array`/`xcalloc_array` allocation with
  `vec![zero B16x4; font_max + 1]`.
- Repointed format dump/undump to the vector storage at `FONT_BASE`.
- Dropped the vector in `tt_cleanup`.
- Removed both `src/ini.rs` `#[link_name="font_check"]` aliases, the raw
  extern declaration from `globals.rs`, and the C definition from
  `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** no row. The source element type is the compound
`b16x4` struct, already represented by Rust's `B16x4`; the move does not erase
a semantic typedef/enum into a primitive.

**longjmp / Drop (§7):** owned vector lives in the `'static` thread-local
`EngineState` and is released explicitly by `tt_cleanup`; abort paths skip Drop
as intended by the existing arena pattern.

**Surprise:** the first dashboard run caught a nested `RefCell` borrow during
format loading because `font_max()` was called inside a `with_state_mut`
allocation closure. Fixed by reading the length before taking the mutable state
borrow, then reran the full dashboard successfully.

**Metrics after this session:**
- `globals.rs` raw extern globals: 205 -> 204 (-1).
- Raw `unsafe { ... }` text-search count: 497 -> 495 (-2, getter/setter
  bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `font_check` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with another bounded `fonts` table subgroup. Good candidates are
  `font_size`/`font_dsize` as a pair (both `scaled_t *`, same init/dump/cleanup
  shape) or another single font table if keeping the slice smaller is preferred.
- Avoid nested `with_state_mut` borrows when allocating: compute capacities via
  accessors before entering the state mutation closure.
- Keep avoiding hot tokenizer/main-loop scratch state; keep `mem` bookkeeping
  scalars with the eventual `Mem` arena move.

## Idiom session 31 — 2026-05-23 — font null-character scalar → FontState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move the single `fonts` null-character word (`null_character`)
out of C globals and into Rust-owned state on `EngineState.fonts`. One
coherent slice, one commit.

**Why this slice:** session 30 handed off `null_character` as the next
bounded `fonts` candidate after the capacity-pair move. It is small enough
to verify its `B16x4` semantics independently before touching the larger
font metric/name arrays and their init/dump/cleanup edges.

**Pre-move grep checks (§11 step 2):**
- (a) C type: `null_character` is `b16x4`.
- (b) cleanup/link aliases: no `src/ini.rs` `#[link_name]` alias.
- (c) C readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: none.

**Work done:**
- Added `null_character` to `FontState`.
- Replaced the derived `FontState` default with an explicit default because
  `B16x4` is a `Copy` struct but does not implement `Default`.
- Repointed `null_character()` and `set_null_character()` to the accessor
  bridge over `EngineState.fonts`.
- Removed the raw extern declaration from `globals.rs` `mod raw` and the C
  definition from `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** no row. The source C type is a four-field
`b16x4` struct, already represented by the Rust `B16x4` struct, so the move
does not erase a semantic typedef/enum into a primitive.

**longjmp / Drop (§7):** scalar state lives in the `'static` thread-local
`EngineState`; no owned heap resource or stack-owned Rust storage is
introduced by this slice.

**Metrics after this session:**
- `globals.rs` raw extern globals: 206 → 205 (−1).
- Raw `unsafe { ... }` text-search count: 499 → 497 (−2, getter/setter
  bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of `null_character` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with another carefully bounded `fonts` subgroup. The remaining
  font arrays (`font_check`, `font_size`, `font_dsize`, `font_params`,
  `font_name`, `font_area`, `font_bc`, `font_ec`, `font_glue`, `font_used`,
  and related arrays) have `src/ini.rs` aliases and format dump/undump edges,
  so pick a coherent ownership slice rather than a loose scalar-style move.
- Keep avoiding the hot tokenizer scalars and `main_*` ligature/main-loop
  scratch scalars; keep `mem` bookkeeping scalars with the eventual `Mem`
  arena move.

## Idiom session 30 — 2026-05-23 — font capacity scalars → FontState

**Type:** execution (workstream 01, Phase-B scalar/link-alias slice).

**Goal:** move the small `fonts` capacity pair (`font_mem_size`,
`font_max`) out of C globals and into Rust-owned fields on
`EngineState.fonts`. One coherent slice, one commit.

**Why this slice:** session 29 established `FontState` and handed off to a
carefully bounded follow-up `fonts` subgroup. These two globals are plain
capacity integers with no cleanup ownership, but they do have `src/ini.rs`
link aliases, so they are a useful bridge step before the larger font arenas.

**Pre-move grep checks (§11 step 2):**
- (a) C types: both `font_mem_size` and `font_max` are `int32_t`.
- (b) cleanup/link aliases: `src/ini.rs` had expected
  `#[link_name]` aliases for both symbols; both were removed/replaced.
- (c) C readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other redeclarations.

**Work done:**
- Added `font_mem_size` and `font_max` fields to `FontState`.
- Repointed `font_mem_size()` and `font_max()` to `EngineState.fonts`, and
  added `set_font_mem_size()` / `set_font_max()` for initialization and format
  loading.
- Replaced the `src/ini.rs` `tre_font_mem_size` / `tre_font_max` aliases with
  the accessor layer in default initialization, format loading, and font-table
  allocation checks.
- Removed the 2 raw extern declarations from `globals.rs` `mod raw` and the 2
  C definitions from `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** no rows. Both moved fields are plain `int32_t`
capacity integers, not semantic typedefs/enums.

**longjmp / Drop (§7):** scalar fields live in the `'static` thread-local
`EngineState`; no owned heap resource or stack-owned Rust storage is introduced
by this slice.

**Metrics after this session:**
- `globals.rs` raw extern globals: 208 → 206 (−2).
- Raw `unsafe { ... }` text-search count: 501 → 499 (−2, both getter bodies
  became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of the 2 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with another bounded `fonts` subgroup. Good next scalar candidate:
  `null_character`, but check its `B16x4` semantics first and keep it separate
  from the large font metric arenas.
- Keep avoiding the hot tokenizer scalars and `main_*` ligature/main-loop
  scratch scalars; keep `mem` bookkeeping scalars with the eventual `Mem`
  arena move.

## Idiom session 29 — 2026-05-23 — font-loading scratch scalars → FontState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move a small, cold `fonts` scalar subgroup
(`loaded_font_mapping`, `loaded_font_flags`, `loaded_font_letter_space`,
`xtx_ligature_present`, `gave_char_warning_help`,
`native_font_type_flag`) out of C globals and into Rust-owned fields on a
new `EngineState.fonts` sub-struct. One coherent slice, one commit.

**Why this slice:** session 28 completed the practical `hyph` ownership
moves and handed off to the next least-coupled non-`misc` cluster. This
font-loading scratch group has no allocation, cleanup, or format aliases;
it is separate from the large font metric arenas and from the hot
per-character `main_*` ligature/main-loop scratch state.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `loaded_font_mapping` is `void *`, `loaded_font_flags` is
  `char`, `loaded_font_letter_space` is `scaled_t`,
  `xtx_ligature_present` and `gave_char_warning_help` are `bool`, and
  `native_font_type_flag` is `int32_t`.
- (b) cleanup/link aliases: no `src/ini.rs` `#[link_name]` aliases for
  any of the 6 symbols.
- (c) C readers: only `xetex/xetex-ini.c` definitions.
- (d) cross-module extern redeclarations: none.

**Work done:**
- Added `FontState` to `src/state.rs` and wired it into `EngineState`.
- Repointed the 12 accessors for the 6 symbols to short
  `with_state` / `with_state_mut` closures.
- Removed the 6 raw extern declarations from `globals.rs` `mod raw` and
  the 6 C definitions from `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** row added for `loaded_font_letter_space`
(`scaled_t` → `Scaled`). The other five moved fields are plain pointer,
char, bool, or `int32_t`, so no rows.

**longjmp / Drop (§7):** scalar fields live in the `'static`
thread-local `EngineState`; no owned heap resource or stack-owned Rust
storage is introduced by this slice.

**Metrics after this session:**
- `globals.rs` raw extern globals: 214 → 208 (−6).
- Raw `unsafe { ... }` text-search count: 513 → 501 (−12, all 12
  accessor bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of the 6 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with another carefully bounded `fonts` subgroup now that
  `EngineState.fonts` exists. Good scalar candidates are
  `font_mem_size`/`font_max` (with their `src/ini.rs` aliases) or
  `null_character` after checking its `B16x4` semantics.
- Keep avoiding the hot tokenizer scalars and `main_*` ligature/main-loop
  scratch scalars; keep `mem` bookkeeping scalars with the eventual `Mem`
  arena move.

## Idiom session 28 — 2026-05-23 — hyph exception arrays → HyphState Vecs

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the remaining dynamically allocated `hyph` exception arrays
(`hyph_word`, `hyph_list`, `hyph_link`) out of C globals and into
Rust-owned `Vec`s on `EngineState.hyph`. One coherent slice, one commit.

**Why this slice:** session 27 moved the dynamic trie arrays and handed off
the exception arrays as the next `hyph` group. These three arrays share init
allocation, format dump/undump participation through accessors, and cleanup
aliases, so they form one ownership slice.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `hyph_word` is `str_number *`, `hyph_list` is `int32_t *`,
  and `hyph_link` is `hyph_pointer *`.
- (b) cleanup/link aliases: init aliases and cleanup aliases existed for all
  3 arrays in `src/ini.rs`; all were removed/replaced in this slice.
- (c) C readers: only `xetex/xetex-ini.c`.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other redeclarations.

**Work done:**
- Added `hyph_word`, `hyph_list`, and `hyph_link` to `HyphState` as `Vec`s.
  Init allocation now sizes all 3 as `hyph_size + 1`, matching the former
  `xmalloc_array(hyph_size)` shape.
- Repointed all 6 exception-table accessors in `src/globals.rs` to safe
  `with_state` / `with_state_mut` indexing.
- Replaced `tt_cleanup` frees for the three arrays with vector resets.
- Removed the 3 raw externs from `globals.rs` `mod raw`, the 6
  `#[link_name]` aliases from `src/ini.rs`, and the 3 C definitions from
  `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** rows added for `hyph_word[]`
(`str_number *` → `Vec<StrNumber>`) and `hyph_link[]`
(`hyph_pointer *` → `Vec<HyphPointer>`). `hyph_list[]` is plain
`int32_t *`, so no row.

**longjmp / Drop (§7):** handled by the established arena pattern. The
owners live in the `'static` thread-local `EngineState`; abort paths do not
jump over stack-owned Rust storage, and cleanup resets the vectors.

**Metrics after this session:**
- `globals.rs` raw extern globals: 217 → 214 (−3).
- Raw `unsafe { ... }` text-search count: 519 → 513 (−6, all 6 array
  accessor bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of the 3 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- The practical `hyph` cluster moves are complete: scalars, fixed arrays,
  dynamic trie arrays, and exception arrays are Rust-owned.
- Pick the next least-coupled non-`misc` cluster from
  `01a-global-inventory.md`. Likely candidates are a bounded `fonts`
  ownership/scalar subgroup or an ownership-aware `eqtb`/`mem` slice. Keep
  avoiding hot tokenizer/main-loop scratch state, and keep `mem` bookkeeping
  scalars with the eventual `Mem` arena move.

## Idiom session 27 — 2026-05-23 — hyph dynamic trie arrays → HyphState Vecs

**Type:** execution (workstream 01, Phase-B ownership slice).

**Goal:** move the 9 dynamically allocated `hyph` trie arrays (`trie_trl`,
`trie_tro`, `trie_trc`, `trie_c`, `trie_o`, `trie_l`, `trie_r`,
`trie_hash`, `trie_taken`) out of C globals and into Rust-owned `Vec`s on
`EngineState.hyph`. One coherent slice, one commit.

**Why this slice:** session 26 moved the fixed-size `hyph` arrays and left
the dynamically allocated trie arrays as the next least-entangled
hyphenation group. These arrays share trie init/format behavior and are
separate from the remaining hyphenation-exception lists
(`hyph_word`/`hyph_list`/`hyph_link`), so they make a coherent ownership
slice.

**Pre-move grep checks (§11 step 2):**
- (a) C types: semantic typedef pointers are `trie_trl`/`trie_tro`/
  `trie_l`/`trie_r`/`trie_hash` (`trie_pointer *`), `trie_c`
  (`packed_UTF16_code *`), and `trie_o` (`trie_opcode *`). Plain pointers
  checked: `trie_trc` (`uint16_t *`) and `trie_taken` (`bool *`).
- (b) cleanup/link aliases: init/format aliases existed for all 9 arrays;
  cleanup aliases existed for `trie_trl`, `trie_tro`, and `trie_trc`. All
  were removed/replaced in this slice.
- (c) C readers: only `xetex/xetex-ini.c` definitions.
- (d) cross-module extern redeclarations: only the expected `src/ini.rs`
  aliases; no other redeclarations.

**Work done:**
- Added the 9 dynamic trie arrays to `HyphState` as `Vec`s. Initex
  allocation now sizes all 9 as `trie_size + 1`, matching the former
  `xmalloc_array(trie_size)` shape.
- Repointed all 18 array accessors in `src/globals.rs` to safe
  `with_state` / `with_state_mut` indexing.
- Replaced format dump/undump aliases for `trie_trl`/`trie_tro`/
  `trie_trc` with short-lived pointers derived from `EngineState.hyph`.
- Replaced `tt_cleanup` frees for `trie_trl`/`trie_tro`/`trie_trc` with
  vector resets, and reset the other six trie build arrays in the same
  cleanup block.
- Removed the 9 raw externs from `globals.rs` `mod raw`, the 12
  `#[link_name]` aliases from `src/ini.rs`, and the 9 C definitions from
  `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** 7 rows added to the Deferred newtype
worklist for `trie_trl[]`, `trie_tro[]`, `trie_c[]`, `trie_o[]`,
`trie_l[]`, `trie_r[]`, and `trie_hash[]`. `trie_trc[]` is plain
`uint16_t *` and `trie_taken[]` is plain `bool *`, so no rows.

**longjmp / Drop (§7):** handled by the established arena pattern. The
owners live in the `'static` thread-local `EngineState`; abort paths do
not jump over stack-owned Rust storage, and cleanup resets the vectors.

**Metrics after this session:**
- `globals.rs` raw extern globals: 226 → 217 (−9).
- Raw `unsafe { ... }` text-search count: 537 → 519 (−18, all 18 array
  accessor bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of the 9 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with the remaining `hyph` exception arrays: `hyph_word`,
  `hyph_list`, and `hyph_link`. They have init/undump allocation and
  cleanup aliases, plus format dump/undump participation, so use the same
  ownership-slice recipe.

## Idiom session 26 — 2026-05-23 — hyph fixed arrays → HyphState Vecs

**Type:** execution (workstream 01, Phase-B fixed-array state slice).

**Goal:** move the 11 fixed-size `hyph` arrays (`hc`, `hu`, `hyf`, `trie_min`, `hyf_distance`, `hyf_num`, `hyf_next`, `op_start`, `trie_used`, `trie_op_lang`, `trie_op_val`) out of C globals and into `EngineState.hyph`. One coherent slice, one commit.

**Why this slice:** session 25 established `EngineState.hyph` with the clean scalar fields, and `STATUS.md` names the `hyph` arrays as the next remaining work. The fixed-size arrays are the least entangled part of the remaining hyphenation state; doing them first avoids mixing format-dump alias rewrites with the later dynamic allocation/cleanup slice.

**Pre-move grep checks (§11 step 2):**
- (a) C types: semantic typedef arrays are `trie_min` (`trie_pointer[65536]`), `hyf_distance`/`hyf_num` (`small_number[TRIE_OP_SIZE + 1]`), `hyf_next`/`trie_used`/`trie_op_val` (`trie_opcode[...]`). Plain arrays checked: `hc`/`hu` (`int32_t`), `hyf`/`trie_op_lang` (`unsigned char`), `op_start` (`int32_t`).
- (b) cleanup/link aliases: `hyf_distance`, `hyf_num`, `hyf_next`, and `trie_used` had `src/ini.rs` format-dump aliases; all were removed/replaced in this slice. No `tt_cleanup` frees.
- (c) C readers: only `xetex/xetex-ini.c` definitions.
- (d) cross-module extern redeclarations: only the four `src/ini.rs` aliases above; no other redeclarations.

**Work done:**
- Added the 11 arrays to `HyphState` as fixed-size, heap-backed `Vec`s. The first attempt used inline fixed arrays, but `engine_rust` exposed a test-thread stack overflow during thread-local default construction; `Vec` keeps the no-realloc fixed-size behavior while moving the storage off the stack.
- Repointed all 22 array accessors in `src/globals.rs` to safe `with_state` / `with_state_mut` indexing.
- Removed the 11 raw externs from `globals.rs` `mod raw` and the 11 C definitions from `xetex/xetex-ini.c`.
- Replaced the four `src/ini.rs` format aliases with short-lived pointers derived from `EngineState.hyph`, and changed `trie_used` dump/undump loops to use the accessor pair.

**Newtype debt (§4 check):** 6 rows added to `Deferred newtype worklist` for `trie_min[]`, `hyf_distance[]`, `hyf_num[]`, `hyf_next[]`, `trie_used[]`, and `trie_op_val[]`. `hc[]`, `hu[]`, `hyf[]`, `op_start[]`, and `trie_op_lang[]` are plain declared C numeric/byte arrays and need no row.

**longjmp / Drop (§7):** N/A for cleanup semantics — these were fixed C/BSS arrays with no `malloc`/`free` edge. The replacement storage lives in the thread-local `EngineState`, so it has the same lifetime pattern as prior Rust-owned state.

**Metrics after this session:**
- `globals.rs` raw extern globals: 237 → 226 (−11).
- Raw `unsafe { ... }` text-search count: 559 → 537 (−22, all 22 array accessor bodies became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4; cargo ran them in order 7/4/29/2).
- `grep` of the 11 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with the remaining `hyph` dynamic arrays: trie arrays (`trie_trl`, `trie_tro`, `trie_trc`, `trie_c`, `trie_o`, `trie_l`, `trie_r`, `trie_hash`, `trie_taken`) and/or hyphenation exception arrays (`hyph_word`, `hyph_list`, `hyph_link`). These have init/undump allocation and cleanup aliases, so use the ownership-slice recipe.

## Idiom session 25 — 2026-05-23 — hyph cluster clean scalars → HyphState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move the 25 clean scalar fields of the `hyph` cluster (including 19 defined in `xetex-ini.c` and 6 defined in `xetex-linebreak.c`) into `EngineState.hyph`. One coherent slice, no commit (per user request).

**Why this slice:**
1. **Next Authoritative Cluster:** As indicated by the "Next action" in `STATUS.md`, the `hyph` cluster is the designated next target since all `input`-cluster arrays are now Rust-owned, and other clean cold scalar groups from other features are exhausted.
2. **Logical Phased Approach:** `STATUS.md` explicitly allows split migrations on `hyph`: *"We can continue Phase-B moves on `hyph` either with the clean scalar fields first or the trie/hyph arrays."*
3. **Safety & Clarity:** Migrating all 25 scalars first cleanly establishes `EngineState.hyph` and its accessors without having to worry about complex `Vec`/alloc/cleanup details of the 10+ dynamically allocated or large fixed arrays in the same commit. This makes the diff reviewable and extremely safe.

**Pre-move grep checks (§11 step 2):**
- (a) C types: 7 variables are semantic types (`hf` is `internal_font_number`, `hyphen_passed` is `small_number`, `max_op_used` is `trie_opcode`, and `trie_max`/`trie_ptr`/`hyph_start`/`hyph_index` are `trie_pointer`).
- (b) cleanup aliases: none.
- (c) C readers: only `xetex-ini.c` / `xetex-linebreak.c` (defining the C globals) compiled by build.rs.
- (d) cross-module extern redeclarations: none.

**Work done:**
- Added `HyphState` sub-struct and the 25 clean scalar fields to `src/state.rs`.
- Repointed all 25 getters and setters in `src/globals.rs` to safe `with_state` / `with_state_mut` closures (removing their inner `unsafe` blocks completely).
- Removed the 25 raw extern definitions from `globals.rs` `mod raw`.
- Converted `tre_trie_size` and `tre_hyph_size` from link-name external C references to safe private module-level Rust `static mut` variables to satisfy the linker after C definitions were removed.
- Initialized `state.hyph.trie_size` and `state.hyph.hyph_size` using `tre_trie_size` and `tre_hyph_size` in `src/ini.rs` `tt_run_engine` initialization path.
- Removed `trie_size` and `hyph_size` external link declarations from `src/ini.rs`.
- Removed all 19 C definitions from `xetex/xetex-ini.c`.
- Removed all 6 C definitions from `xetex/xetex-linebreak.c`.

**Newtype debt (§4 check):** 7 rows added to `Deferred newtype worklist` in `01a-global-inventory.md` for semantic-typed fields (`hf`, `hyphen_passed`, `trie_max`, `trie_ptr`, `max_op_used`, `hyph_start`, `hyph_index`). Plain variables need no rows.

**longjmp / Drop (§7):** N/A — plain scalar/pointer fields with no owned heap storage, no `Drop` edge. Thread-local owner unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 262 → 237 (−25).
- Raw `unsafe { ... }` text-search count: 607 → 559 (−48, 24 getter/setter accessor pairs became fully safe).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4).
- `grep` of the 25 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with execution of workstream 01 by targeting `hyph` arrays (`hc`, `hu`, `hyf`, `trie_min`, `hyf_distance`, `hyf_num`, `hyf_next`, `op_start`, `trie_used`, `trie_op_lang`, `trie_op_val`), applying the static array mapping or dynamic allocations patterns established in prior sessions.

---

## Idiom session 24 — 2026-05-23 — strings cluster arenas and scalars → StringsState

**Type:** execution (workstream 01, Phase-B scalar/ownership slice).

**Goal:** move the 13 symbols of the `strings` cluster (comprising arrays `str_start` / `str_pool` and 11 scalar fields: `str_ptr`, `max_strings`, `init_str_ptr`, `init_pool_ptr`, `job_name`, `texmf_log_name`, `string_vacancies`, `pool_size`, `pool_ptr`, `format_ident`, `edit_name_start`) into `EngineState.strings`. One coherent slice, one commit.

**Why this slice:** This is the designated next action from STATUS.md and the handoff of session 23. Resolving the strings cluster represents the first ownership-transfer slice of format-dumped/undumped arrays, establishing a vital precedent for future large arrays like `eqtb` and `mem`.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `str_start` is `pool_pointer *` (*mut i32), `str_pool` is `packed_UTF16_code *` (*mut u16). The 11 scalars are primarily `str_number` or `pool_pointer` (meaning we add deferred newtype rows for 8 of them: `str_ptr`, `init_str_ptr`, `init_pool_ptr`, `job_name`, `texmf_log_name`, `pool_ptr`, `format_ident`, `edit_name_start`).
- (b) cleanup aliases: 9 occurrences in `src/ini.rs` (tre_pool_size, tre_string_vacancies, tre_max_strings, tre_str_start, tre_str_pool, tre_init_str_ptr, tre_init_pool_ptr, tt_cleanup_str_start, tt_cleanup_str_pool) - all removed/redirected to StringsState.
- (c) C readers: only `xetex-ini.c` (defining the C globals) compiled by build.rs.
- (d) cross-module extern redeclarations: none.

**Work done:**
- Added `StringsState` sub-struct and the 13 fields to `src/state.rs`.
- Repointed all 14 getters and setters in `src/globals.rs` to safe `with_state` / `with_state_mut` closures (only `str_pool_ptr` uses an inner `unsafe` block for raw pointer arithmetic).
- Replaced C `malloc` allocations in `src/ini.rs` (initex alloc at line 3355, undump alloc at line 2680) with safe `vec![0; n + 1]` allocations stored on `state.strings.str_start` and `state.strings.str_pool`.
- Replaced `libc::free` cleanup in `src/ini.rs` (undump cleanup at line 2572, tt_cleanup at line 3784) with vector resets `Vec::new()`.
- Pre-evaluated `str_ptr()` and `pool_ptr()` calls in `src/ini.rs` outside the mutable closure borrow `with_state_mut` block to avoid RefCell nested borrow panics.
- Removed all 9 `#[link_name]` aliases from `src/ini.rs`.
- Converted `tre_pool_size`, `tre_string_vacancies`, and `tre_max_strings` from link-name external C references to safe private module-level Rust `static mut` variables to satisfy the linker after C definitions were removed.
- Removed all 13 C definitions from `xetex/xetex-ini.c`.

**Newtype debt (§4 check):** 8 rows added to `Deferred newtype worklist` in `01a-global-inventory.md` for semantic-typed scalars. Genuinely-numeric variables (`max_strings`, `string_vacancies`, `pool_size`) need no rows.

**longjmp / Drop (§7):** N/A — arrays are owned by the thread-local state, meaning they persist past any longjmp/abort boundaries and are cleanly collected when the thread terminates, matching prior behavior.

**Metrics after this session:**
- `globals.rs` raw extern globals: 275 → 262 (−13).
- Raw `unsafe { ... }` text-search count: 628 → 607 (−21).
- `#[link_name]` aliases: removed 9 aliases from `src/ini.rs`.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4).
- `grep` of the 13 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Continue with execution of workstream 01 by targeting `hyph` (hyphenation and trie arrays), starting either with hyphenation scalars or trie pointer/trie-state arrays (`trie_trl`, `trie_tro`, `trie_trc`, `trie_c`, `trie_o`, `trie_l`, `trie_r`, `trie_hash`, `trie_taken`, etc.), applying the same init/undump/cleanup pattern established in session 24.

---

## Idiom session 23 — 2026-05-23 — shipout file-handles (rust_stdout/log_file/current_diagnostic) → ShipoutState

**Type:** execution (workstream 01, Phase-B scalar/ownership slice).

**Goal:** move the 3 remaining `shipout` file-handle state variables `rust_stdout` (`usize`/`rust_output_handle_t`), `log_file` (`usize`/`rust_output_handle_t`), and `current_diagnostic` (`*mut c_void`/`ttbc_diagnostic_t *`) into `EngineState.shipout`. One coherent slice (the remaining file-handle members of the `shipout` group), one commit (once reviewed).

**Why this slice:** Clean cold `scanner` scalar filler is fully exhausted as of session 22. The other remaining ownership-bearing options are more involved (e.g. `strings` touches the format dump/undump paths). Moving these 3 completes the BSS/extern file-handle state of `shipout`, resolving `rust_stdout`'s cleanup alias and moving `current_diagnostic` out of C-side storage `xetex-output.c`.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `rust_stdout` and `log_file` are `rust_output_handle_t` in `xetex/xetex-xetexd.h` — type is a `uintptr_t` typedef, meaning we add newtype debt rows to the Deferred newtype worklist for both `rust_stdout` and `log_file`. `current_diagnostic` is `ttbc_diagnostic_t *`, which is a plain opaque pointer (like `input_file`'s handles) and needs no row.
- (b) cleanup aliases: `rust_stdout` has a `#[link_name = "rust_stdout"]` alias in `src/ini.rs:1781` with the name `tre_rust_stdout`, which is initialized but has no cleanup/free call. This alias was removed and replaced with a direct write through `set_rust_stdout` in the same commit.
- (c) C readers: only `xetex/xetex-ini.c` (defining `rust_stdout` and `log_file`) and `xetex/xetex-output.c` (defining `current_diagnostic`).
- (d) cross-module redeclarations: none.

**Work done:**
- Added the 3 fields to `ShipoutState` in `src/state.rs` (`rust_stdout: usize`, `log_file: usize`, `current_diagnostic: *mut std::ffi::c_void`); `#[derive(Default)]` covers them.
- Repointed all 5 accessors in `globals.rs` (`rust_stdout`/`set_rust_stdout`, `log_file`/`set_log_file`, `current_diagnostic`/`set_current_diagnostic`) to `with_state`/`with_state_mut` closures. All 5 are now fully safe (no `unsafe`).
- Removed `#[link_name = "rust_stdout"]` alias `tre_rust_stdout` from `src/ini.rs` and replaced its initialization in `tt_run_engine` with a safe call to the `set_rust_stdout` accessor.
- Removed the 3 raw extern definitions from `globals.rs` `mod raw`.
- Removed C definitions of `rust_stdout` and `log_file` from `xetex/xetex-ini.c`.
- Removed C definition of `current_diagnostic` from `xetex/xetex-output.c`.

**Newtype debt (§4 check):** Two rows added to `Deferred newtype worklist` in `01a-global-inventory.md`: `rust_stdout` (`rust_output_handle_t` → `OutputHandle`) and `log_file` (`rust_output_handle_t` → `OutputHandle`/`Option<OutputId>`). `current_diagnostic` is a plain pointer, no newtype debt (checked).

**longjmp / Drop (§7):** N/A — plain scalar/pointer fields with no owned heap storage, no `Drop` edge. Thread-local owner unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 278 → 275 (−3).
- Raw `unsafe { ... }` text-search count: 633 → 628 (−5, all 5 accessors became fully safe).
- `#[link_name]` aliases: removed `rust_stdout` alias.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of the 3 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Reconcile `strings` (`str_start` / `str_pool`), which **are** serialized in the format dump/undump paths — this is the first ownership slice that touches the format paths, more involved than the input arrays.

---

## Idiom session 22 — 2026-05-23 — cold sparse-array register scalars (sa_chain/sa_level/max_reg_num) → ScannerState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move the 3 clean cold sparse-array register scalars `sa_chain`
(`int32_t`/i32), `sa_level` (`uint16_t`/u16), `max_reg_num` (`int32_t`/i32) into
`EngineState.scanner`. One coherent slice (the clean cold members of the `sa_*`
group), one commit.

**Why this slice:** clean cold `scanner` scalar filler is otherwise exhausted,
and the remaining ownership slices (`strings` touches format dump/undump;
`shipout` file-handle state has cleanup edges) are more involved. The `sa_*`
sparse-array group's three plain-integer members are the lowest-risk remainder:
tightly scoped (`sa_chain` 16 uses, `sa_level` 8, `max_reg_num` 7), pure
`\global`-register sparse-array machinery, never on the tokenizer/per-character
hot path. The other `sa_*` members were deliberately left behind: `sa_root` has
a `#[link_name]` alias (ownership slice), `sa_null` is a `MemoryWord`,
`max_reg_help_line` is a `*const c_char`, and `cur_ptr` (148 uses across
save.rs/pagebuilder.rs/scan.rs) is too broad to bridge without benchmarking.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `extern int32_t sa_chain`, `extern uint16_t sa_level`,
  `extern int32_t max_reg_num` in `xetex-xetexd.h` — all plain integer types,
  **no newtype debt** (`sa_level` is conceptually a quarterword level but its
  declared C type is plain `uint16_t`, so per the session-20/21 precedent no row
  is added).
- (b) cleanup aliases: none for any of the 3 → scalar slice, not ownership.
  (`sa_root`, excluded, *does* have one.)
- (c) C readers: only `xetex/xetex-ini.c` (the definitions).
- (d) cross-module redeclarations: none.

**Work done:**
- Added the 3 fields to `ScannerState` in `src/state.rs` (`sa_chain: i32`,
  `sa_level: u16`, `max_reg_num: i32`); `#[derive(Default)]` covers them.
- Repointed all 6 accessors in `globals.rs`
  (`sa_chain`/`set_sa_chain`, `sa_level`/`set_sa_level`,
  `max_reg_num`/`set_max_reg_num`) from `unsafe { raw::… }` to
  `with_state`/`with_state_mut` closures. All 6 are now fully safe (no `unsafe`).
- Removed the 3 raw externs from `globals.rs` `mod raw` (interleaved with the
  kept `sa_root` array and `max_reg_help_line` pointer, removed individually)
  and the 3 C def lines (`max_reg_num` at 245, `sa_chain`/`sa_level` at 250–251)
  from `xetex/xetex-ini.c`. No allocation/cleanup edits (plain BSS scalars).

**Newtype debt (§4 check):** none — all three are plain `int32_t`/`uint16_t`,
no semantic typedef (checked).

**longjmp / Drop (§7):** N/A — plain scalar fields, no owned heap storage, no
`Drop` edge. Thread-local owner unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 281 → 278 (−3).
- Raw `unsafe { ... }` text-search count: 639 → 633 (−6, all 6 accessors became
  fully safe).
- `#[link_name]` aliases: unchanged (this slice had none).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of the 3 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Clean cold `scanner` scalar filler is now **exhausted**. The remaining `sa_*`
  members are all messier: `sa_root` (link alias → ownership slice), `sa_null`
  (`MemoryWord`), `max_reg_help_line` (`*const c_char` pointer), `cur_ptr`
  (148 uses, too broad — bridge only with benchmarking or in Phase C).
- Remaining ownership-bearing options (rough difficulty order): `strings`
  (`str_start`/`str_pool`, **serialized in format dump/undump** — first
  format-path slice, most involved); remaining `shipout` file-*handle* state
  (`rust_stdout` has a cleanup alias → ownership slice; `log_file`;
  `current_diagnostic`, C-defined in `xetex-output.c`).
- Reminder: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
  `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) or the `main_*` ligature/main-loop
  scratch scalars (per-character hot path); the `mem` bookkeeping scalars go
  with the `Mem` arena move (§5).

## Idiom session 21 — 2026-05-23 — cold scanner scalars (shown_mode/mag_set/radix/expand_depth_count) → ScannerState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move the 4 clean cold scanner scratch scalars `shown_mode`
(`short`/i16), `mag_set` (`int32_t`/i32), `radix` (`small_number`, stored u8),
`expand_depth_count` (`int32_t`/i32) into `EngineState.scanner`. One coherent
slice (clean cold scanner scalars), one commit.

**Why this slice:** the remaining ownership slices (`strings` touches the
format dump/undump; `shipout` file-handle state has cleanup edges) are more
involved. These 4 are the confirmed-clean cold scanner scalars carried as
low-risk filler since session 20: `shown_mode` is the show-mode latch for
`show_cur_cmd_chr`; `mag_set` is the once-per-job magnification latch; `radix`
is the scanner number base (8/10/16); `expand_depth_count` is the
`\expanded`/expression-depth guard counter. None are on the tokenizer or
per-character hot path. They share a contiguous-ish declaration region in
`xetex-ini.c` and `mod raw`, so the move is clean.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `extern short shown_mode`, `extern int32_t mag_set`,
  `extern small_number radix`, `extern int32_t expand_depth_count` in
  `xetex-xetexd.h`. `small_number` is `typedef short` (`xetex-xetexd.h:50`) — a
  semantic typedef → **newtype debt for `radix`**. The other three are plain
  `short`/`int32_t` → no debt (`shown_mode` holds a mode value but its declared
  C type is plain `short`, not a mode typedef/enum, so per the session-20
  precedent no row is added).
- (b) cleanup aliases: none for any of the 4 → scalar slice, not ownership.
- (c) C readers: only `xetex/xetex-ini.c` (the definitions).
- (d) cross-module redeclarations: none.

**Work done:**
- Added the 4 fields to `ScannerState` in `src/state.rs` with their existing
  bridge widths (`shown_mode: i16`, `mag_set: i32`, `radix: u8`,
  `expand_depth_count: i32`); `#[derive(Default)]` covers them.
- Repointed all 8 accessors in `globals.rs` (`shown_mode`/`set_shown_mode`,
  `mag_set`/`set_mag_set`, `radix`/`set_radix`,
  `expand_depth_count`/`set_expand_depth_count`) from `unsafe { raw::… }` to
  `with_state`/`with_state_mut` closures. All 8 are now fully safe (no `unsafe`).
- Removed the 4 raw externs from `globals.rs` `mod raw` and the 4 C def lines
  (75/81/102/110) from `xetex/xetex-ini.c`, each interleaved with kept symbols
  so removed individually. No allocation/cleanup edits (plain BSS scalars).

**Newtype debt (§4 check):** one row added — `radix` (`small_number` →
`SmallNumber`/`Radix`). `shown_mode`/`mag_set`/`expand_depth_count` are plain
`short`/`int32_t`, no rows (checked).

**longjmp / Drop (§7):** N/A — plain scalar fields, no owned heap storage, no
`Drop` edge. Thread-local owner unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 285 → 281 (−4).
- Raw `unsafe { ... }` text-search count: 647 → 639 (−8, all 8 accessors became
  fully safe).
- `#[link_name]` aliases: unchanged (this slice had none).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of the 4 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Remaining ownership-bearing options (rough difficulty order): `strings`
  (`str_start`/`str_pool`, **serialized in format dump/undump** — first
  format-path slice, most involved); remaining `shipout` file-*handle* state
  (`rust_stdout` has a cleanup alias → ownership slice; `log_file`;
  `current_diagnostic`, C-defined in `xetex-output.c`).
- Clean cold `scanner` scalar filler is now nearly exhausted. The `sa_*`
  sparse-array group (`cur_ptr`, `sa_null`, `sa_root`, `sa_chain`, `sa_level`,
  `max_reg_num`, `max_reg_help_line`) remains but is messier: `sa_root` has a
  link alias (ownership), `sa_null` is a `MemoryWord`, `max_reg_help_line` is a
  `*const c_char`.
- Reminder: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
  `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) or the `main_*` ligature/main-loop
  scratch scalars (per-character hot path); the `mem` bookkeeping scalars go
  with the `Mem` arena move (§5).

## Idiom session 20 — 2026-05-23 — alignment state scalars → ScannerState

**Type:** execution (workstream 01, Phase-B scalar slice).

**Goal:** move the 8 alignment (`\halign`/`\valign`) scratch scalars
`cur_align`, `cur_span`, `cur_loop`, `align_ptr`, `cur_head`, `cur_tail`,
`cur_pre_head`, `cur_pre_tail` into `EngineState.scanner`. One coherent slice
(the alignment state group), one commit.

**Why this slice:** with all `input`-cluster arrays Rust-owned and the
`shipout` write-stream arrays done, the remaining ownership slices (`strings`
touches the format dump/undump; the `shipout` file-handle state has cleanup
edges) are more involved. The alignment scalars are a semantically coherent
cold group — they are touched per-alignment-cell, never on the per-token or
per-character hot path (usage counts 15–51 across the crate, all in
`align`/`scan` paths), so the thread-local bridge has no hot-path cost. They
share a contiguous declaration block in `xetex-ini.c` and `mod raw`, so the
move is clean. Lowest-risk way to keep advancing the `scanner` cluster.

**Pre-move grep checks (§11 step 2):**
- (a) C types: all 8 are plain `extern int32_t` in `xetex-xetexd.h`
  (`cur_head, cur_tail` and `cur_pre_head, cur_pre_tail` are paired decls).
  No semantic typedef → **no newtype debt** for any of them (checked). They
  are TeX mem "pointers" in spirit, but the declared C type is `int32_t`, so
  the §4 typedef-debt rule does not apply; their eventual `MemPtr`/`Halfword`
  shape is an arena/Phase-C concern, not a typedef-debt row.
- (b) cleanup aliases: none (`link_name = "…"` empty for all 8) → scalar
  slice, not an ownership slice.
- (c) C readers: only `xetex/xetex-ini.c` (the definitions).
- (d) cross-module redeclarations: none.

**Work done:**
- Added the 8 `i32` fields to `ScannerState` in `src/state.rs`
  (`#[derive(Default)]` covers them).
- Repointed all 16 accessors in `globals.rs`
  (`cur_align`/`set_cur_align`, …, `cur_pre_tail`/`set_cur_pre_tail`; note the
  `cur_tail` getter/setter are named `cur_tail_a`/`set_cur_tail_a`) from
  `unsafe { raw::… }` to `with_state`/`with_state_mut` closures. All 16 are
  now fully safe (no `unsafe`).
- Removed the 8 raw externs from `globals.rs` `mod raw` and the 6 C def lines
  (177–182) from `xetex/xetex-ini.c`. No allocation/cleanup edits (plain
  BSS scalars).

**Newtype debt (§4 check):** none — all 8 are plain `int32_t` in the header
(checked). No worklist rows added.

**longjmp / Drop (§7):** N/A — plain scalar fields, no owned heap storage, no
`Drop` edge. Thread-local owner unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 293 → 285 (−8).
- Raw `unsafe { ... }` text-search count: 663 → 647 (−16, all 16 accessors
  became fully safe).
- `#[link_name]` aliases: unchanged (this slice had none).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of the 8 symbols in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Remaining ownership-bearing options (rough difficulty order): `strings`
  (`str_start`/`str_pool`, **serialized in format dump/undump** — first
  format-path slice, most involved); remaining `shipout` file-*handle* state
  (`rust_stdout` has a cleanup alias → ownership slice; `log_file`;
  `current_diagnostic`, C-defined in `xetex-output.c`).
- More clean cold `scanner` scalars remain as low-risk filler — confirmed
  clean candidates from prior sessions: `shown_mode`, `mag_set`, `radix`,
  `expand_depth_count`. The `sa_*` sparse-array group (`cur_ptr`, `sa_null`,
  `sa_root`, `sa_chain`, `sa_level`, `max_reg_num`, `max_reg_help_line`) is
  also `scanner` but messier: `sa_root` has a link alias (ownership), `sa_null`
  is a `MemoryWord`, `max_reg_help_line` is a `*const c_char`.
- Reminder: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
  `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) or the `main_*` ligature/main-loop
  scratch scalars (per-character hot path); the `mem` bookkeeping scalars go
  with the `Mem` arena move (§5).

## Idiom session 19 — 2026-05-23 — shipout write_file/write_open arrays → ShipoutState

**Type:** execution (workstream 01, Phase-B scalar-style slice).

**Goal:** move the two write-stream arrays `write_file`
(`rust_output_handle_t[16]`) and `write_open` (`bool[18]`) into
`EngineState.shipout`. One coherent slice (the shipout write-stream state),
one commit.

**Why this slice:** with all `input`-cluster arrays now Rust-owned (sessions
11–18), the remaining ownership-bearing options are `strings` (touches the
format dump/undump paths — most involved) and the rest of the `shipout`
file-handle state. `write_file`/`write_open` are **inline fixed C arrays**
(`[16]`/`[18]` in BSS, not `malloc`'d), so they move exactly like scalars:
no `ini.rs` allocation, no `tt_cleanup` free, no `#[link_name]` cleanup
alias. Lowest-risk way to keep advancing the shipout cluster. The
file-*handle* state with init/cleanup edges (`rust_stdout`, `log_file`,
`current_diagnostic`) stays behind for a later slice.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `rust_output_handle_t write_file[16]` and `bool write_open[18]`
  in `xetex-xetexd.h:691–692`. `rust_output_handle_t` is
  `typedef uintptr_t` (`tectonic_bridge_core_generated.h:9`; bridge_core also
  names `Option_OutputId = rust_output_handle_t`) — a nullable output-stream
  handle (0 = none), a semantic typedef → **newtype debt** for `write_file`.
  `write_open` is plain `bool` → no row.
- (b) cleanup aliases: none (`link_name = "write_file"|"write_open"` empty).
- (c) C readers: only `xetex/xetex-ini.c` (the definitions). Not in the
  format dump/undump (grep for dump/undump empty).
- (d) cross-module redeclarations: none.

**Work done:**
- Added `write_file: [usize; 16]` and `write_open: [bool; 18]` to
  `ShipoutState` in `src/state.rs` (`#[derive(Default)]` covers both — arrays
  of length ≤ 32 implement `Default`).
- Repointed the 4 accessors in `globals.rs` (`write_file`, `write_open`,
  `set_write_file`, `set_write_open`) from `unsafe { raw::… }` to
  `with_state`/`with_state_mut` closures. All 4 are now fully safe (no
  `unsafe`).
- Removed the two raw externs from `globals.rs` `mod raw` and the two C defs
  from `xetex/xetex-ini.c`. No allocation/cleanup edits needed (inline
  arrays).

**Newtype debt (§4 check):** `write_file[]` → worklist row added
(`rust_output_handle_t` → `OutputHandle`/`Option<OutputId>`). `write_open[]`
is plain `bool`, no row (checked).

**longjmp / Drop (§7):** N/A for this slice — these are plain inline fields,
no owned heap storage, no `Drop` edge. The `EngineState` thread-local owner
is unchanged.

**Metrics after this session:**
- `globals.rs` raw extern globals: 295 → 293 (−2).
- Raw `unsafe { ... }` text-search count: 667 → 663 (−4, all 4 accessors
  became fully safe).
- `#[link_name]` aliases: unchanged (this slice had none).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `write_file`/`write_open` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Remaining ownership-bearing options (rough difficulty order):
  - `strings` (`str_start` / `str_pool`): **serialized in the format
    dump/undump**, so that slice must also reconcile the load/dump paths
    (first ownership slice touching the format paths — more involved).
  - remaining `shipout` file-*handle* state: `rust_stdout` (has a cleanup
    alias → ownership slice), `log_file`, `current_diagnostic` (C-defined in
    `xetex-output.c`, not `xetex-ini.c`).
- Clean (no-cleanup-alias) non-hot `scanner` scalars also still available as
  low-risk filler (`shown_mode`, `mag_set`, `radix`, `expand_depth_count`, …;
  all confirmed clean this session — only `xetex-ini.c` defines them, no
  cleanup alias, no cross-module redecl).
- Reminder: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
  `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`); the `mem` bookkeeping scalars
  go with the `Mem` arena move (§5), not as a loose slice.

## Idiom session 18 — 2026-05-23 — nest array → Rust-owned Vec

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** move the `nest` (`list_state_record * == *mut ListStateRecord`)
into a Rust-owned `Vec<ListStateRecord>` on `EngineState.input`. One array,
one commit. **Last** remaining `input`-cluster array — with this slice all
input arrays are Rust-owned.

**Why nest now:** it was the last input array left; it has no
pointer-escaping accessor (unlike `buffer`/`input_file`), so the move is
purely "index the `Vec`" per accessor — the simplest remaining shape.
`ListStateRecord` already exists in `mem_types.rs` as a
`#[repr(C)] #[derive(Copy, Clone)]` struct, so `Vec<ListStateRecord>`
derives `Default` (empty) and `InputClusterState` still derives `Default`
with no hand-written impl. Only C reference is its `xetex-ini.c` definition;
not serialized in the format dump/undump.

**Pre-move grep checks (§11 step 2):** (a) declared `list_state_record
*nest;` in `xetex-xetexd.h:476`. `list_state_record` is a struct typedef,
already mirrored 1:1 by the Rust `ListStateRecord` — a compound struct, not
a scalar semantic typedef, so **no** newtype debt (type info preserved in
the struct, same reasoning as session-16 `input_stack`/`InputState`). (b)
two `link_name = "nest"` aliases in `ini.rs` (alloc `tre_nest` + cleanup
`tt_cleanup_nest`) → ownership slice. (c) only `xetex-ini.c` references it.
(d) no foreign-module redeclaration (only the two `ini.rs` aliases being
removed).

**Work done:**
- Added `nest: Vec<crate::mem_types::ListStateRecord>` to
  `InputClusterState` in `src/state.rs` (`#[derive(Default)]` still covers
  the struct — empty Vec).
- Repointed the 12 `raw::nest` accessors in `globals.rs`:
  `restore_cur_list_from_nest`/`save_cur_list_to_nest` copy the whole
  `ListStateRecord` to/from the separate `cur_list` static (which is **not**
  part of this slice — it stays a raw global); the 10 field-projection
  accessors (`nest_mode`, `nest_tail`, `set_nest_tail`, `nest_e_tex_aux`,
  `nest_prev_graf`, `set_nest_prev_graf`, `nest_head`, `nest_mode_line`,
  `nest_aux_b32s1`, `nest_aux_b32s0`) index the `Vec` via
  `with_state`/`with_state_mut`. The two `aux.b32.*` reads keep an inner
  `unsafe` (the `MemoryWord` union projection); the other 8 field accessors
  are now fully safe. `nest_ptr`/`max_nest_stack`/`nest_size` are separate
  scalars and stay behind.
- Allocation (`ini.rs`): replaced
  `tre_nest = xmalloc_array::<ListStateRecord>(tre_nest_size)` with a
  `with_state_mut` block writing `vec![<zero ListStateRecord>; nest_size + 1]`
  (matching the former `n + 1` shape; zero word built explicitly since
  `MemoryWord` has no `Default`).
- Cleanup (`tt_cleanup`): dropped `nest` (`= Vec::new()`) folded into the
  existing input cleanup block instead of `libc::free`.
- Removed both `#[link_name = "nest"]` alias decls (alloc + cleanup), the
  raw extern in `globals.rs`, and the C def in `xetex-ini.c`.

**Newtype debt (§4 check):** element type is the `list_state_record` struct,
mirrored 1:1 by `ListStateRecord` — a compound struct, not a scalar typedef
collapse, so **no** worklist row (checked, none needed). The per-field
accessor return types (`mode: i16`, the `i32` aux/graf/head fields) are
unchanged by this slice and were already the accessor shape before.

**longjmp / Drop (§7, handled):** identical to sessions 11–17 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort never
jumps over the `Vec`; `Drop` doesn't run on abort (matches prior C).
`tt_cleanup` is the normal teardown and now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 296 → 295.
- Raw `unsafe { ... }` text-search count: 675 → 667 (8 field accessors became
  fully safe; 4 unsafe remain across the nest accessors — 2 aux-union reads
  + 2 `cur_list` copies).
- `#[link_name]` aliases in `ini.rs`: −2.
- 12 Rust-owned arena arrays total (save_stack, param_stack, the 5 session-13
  line/group stacks, eof_seen, buffer, input_stack, input_file, + nest). **All
  `input`-cluster arrays are now Rust-owned.**

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `nest` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- **No `input` arrays remain** — the input cluster's arrays are fully
  Rust-owned. Remaining ownership-bearing options (rough difficulty order):
  - `strings` (`str_start` / `str_pool`): **serialized in the format
    dump/undump**, so that slice must also reconcile the load/dump paths
    (the first ownership slice that touches the format paths — more involved
    than the input arrays).
  - remaining `shipout` file-handle state: `rust_stdout` (has a cleanup
    alias), `write_file`/`write_open` (inline arrays, movable like scalars),
    `current_diagnostic` (C-defined in `xetex-output.c`, not `xetex-ini.c`).
- Clean (no-cleanup-alias) non-hot `scanner` scalars also still available as
  low-risk filler (`shown_mode`, `mag_set`, `radix`, `expand_depth_count`, …).
- Reminder: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
  `cur_chr`, `cur_tok`, `cur_cs`, `cur_val`); the `mem` bookkeeping scalars
  go with the `Mem` arena move (§5), not as a loose slice.

## Idiom session 17 — 2026-05-23 — input_file array → Rust-owned Vec

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** move the `input_file` (`UFILE ** == *mut *mut c_void`) into a
Rust-owned `Vec<*mut libc::c_void>` on `EngineState.input`. One array, one
commit. Second-to-last `input`-cluster array.

**Why input_file now:** of the two arrays left it has the fewest accessors
(only three: `input_file`, `set_input_file`, and the pointer-escaping
`input_file_ptr`), and that pointer-escape is the exact `buffer_ptr` pattern
already proven in session 15 (`as_mut_ptr().add(k)` past the borrow, valid
because the `'static` thread-local owner never reallocates). `nest` is left
for last (~11 field-projection accessors, no pointer escape). The element is a
plain `UFILE*` handle stored as `*mut c_void` (avoids a cyclic dep on io.rs),
so `Vec<*mut c_void>` derives `Default` (empty) and the `InputClusterState`
still derives `Default` with no hand-written impl. Only C reference is its
`xetex-ini.c` definition; not serialized in the format dump/undump.

**Pre-move grep checks (§11 step 2):** (a) declared `extern UFILE
**input_file;` in `xetex-xetexd.h:509`. `UFILE*` is an opaque file-handle
pointer, not a scalar semantic typedef — **no** newtype debt (raw pointer, no
type info lost). (b) two `link_name = "input_file"` aliases in `ini.rs` (alloc
`tre_input_file` + cleanup `tt_cleanup_input_file`) → ownership slice. (c) only
`xetex-ini.c` references it. (d) no foreign-module redeclaration (the only hits
were the two `ini.rs` aliases being removed).

**Work done:**
- Added `input_file: Vec<*mut libc::c_void>` to `InputClusterState` in
  `src/state.rs` (`#[derive(Default)]` still covers the struct — empty Vec).
- Repointed the three accessors in `globals.rs`: `input_file(k)` /
  `set_input_file(k, v)` index the `Vec` via `with_state`/`with_state_mut`
  (no `unsafe`); `input_file_ptr(k)` (still a safe `fn` with an inner `unsafe`
  block, as before) returns `as_mut_ptr().add(k)` cast to
  `*mut *mut crate::io::UFile` — the session-15 `buffer_ptr` shape.
- Allocation (`ini.rs`): replaced
  `tre_input_file = xmalloc_array::<*mut c_void>(tre_max_in_open)` with a
  `with_state_mut` block writing `vec![null_mut(); max_in_open + 1]` (matching
  the former `n + 1` shape).
- Cleanup (`tt_cleanup`): dropped `input_file` (`= Vec::new()`) folded into the
  existing input cleanup block instead of `libc::free`.
- Removed both `#[link_name = "input_file"]` alias decls (alloc + cleanup), the
  raw extern in `globals.rs`, and the C def in `xetex-ini.c`.

**Newtype debt (§4 check):** element type is `UFILE*`, an opaque file handle
stored as `*mut c_void` — a plain pointer, no semantic typedef collapse, so
**no** worklist row (checked, none needed).

**longjmp / Drop (§7, handled):** identical to sessions 11–16 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort never
jumps over the `Vec`; `Drop` doesn't run on abort (matches prior C).
`tt_cleanup` is the normal teardown and now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 297 → 296.
- Raw `unsafe { ... }` text-search count: 677 → 675.
- `#[link_name]` aliases in `ini.rs`: −2.
- 11 Rust-owned arena arrays total (save_stack, param_stack, the 5 session-13
  line/group stacks, eof_seen, buffer, input_stack, + input_file).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `input_file` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- One input array remains: `nest` (`list_state_record * == *mut
  ListStateRecord`). It has ~11 field-projection accessors to repoint
  (`nest_mode`, `nest_tail`, `set_nest_tail`, `nest_e_tex_aux`,
  `nest_aux_b32s1`, `nest_prev_graf`, `set_nest_prev_graf`, `nest_head`,
  `nest_mode_line`, `nest_aux_b32s0`, …) but **no** pointer escape, so the move
  is purely "index the `Vec`" per accessor. `ListStateRecord` already exists in
  `mem_types.rs` — check it derives `Copy`/`Clone` for the `vec![<zero>; n]`
  init (sized `nest_size + 1`). Two alias decls (alloc `tre_nest` + cleanup
  `tt_cleanup_nest`), one raw extern, one C def to remove. Not in format dump.
  As a struct element, the array itself is no newtype debt — but check the
  per-field accessors' return types (`nest_mode` is `i16` mode, `nest_*` aux
  fields) for semantic typedefs and add worklist rows if any.
- After `nest`, **all** `input`-cluster arrays are Rust-owned. Then `strings`
  (`str_start`/`str_pool`) — but those **are** in the format dump/undump, so
  that slice must reconcile the load/dump paths.

## Idiom session 16 — 2026-05-23 — input_stack array → Rust-owned Vec

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** move the `input_stack` (`input_state_t * == InputState *`) into a
Rust-owned `Vec<InputState>` on `EngineState.input`. One array, one commit.

**Why input_stack now:** of the three remaining input arrays it is the
lowest-difficulty — only **two** accessors (`input_stack(k)` full-struct
copy-out + `set_input_stack(k, v)` full-struct assign), no pointer-escaping
accessor like `buffer_ptr`/`input_file_ptr`, so the move is purely "index
the `Vec`". `InputState` already exists as a `#[derive(Copy, Clone)]` struct
in `globals.rs`, so `Vec<InputState>` derives `Default` (empty vec) and the
`InputClusterState` still derives `Default` with no hand-written impl. The
only C reference is its `xetex-ini.c` definition; it is **not** serialized in
the format dump/undump. (`input_file` and `nest` are left for follow-up:
`input_file` has a pointer-escaping `input_file_ptr` accessor for `u_open_in`,
and `nest` has ~12 field-projection accessors.)

**Pre-move grep checks (§11 step 2):** (a) declared `extern input_state_t
*input_stack;` in `xetex-xetexd.h:503`. `input_state_t` is a struct typedef,
already mirrored 1:1 by the Rust `InputState` — a compound struct, not a
scalar semantic typedef, so **no** newtype debt (the type info is preserved
in the struct, unlike a `selector_t`→`i32` collapse). (b) two
`link_name = "input_stack"` aliases in `ini.rs` (alloc `tre_input_stack` +
cleanup `tt_cleanup_input_stack`) → ownership slice. (c) only `xetex-ini.c`
references it. (d) no foreign-module `extern`/`static mut` redeclaration (the
only hits were the two `ini.rs` aliases being removed).

**Work done:**
- Added `input_stack: Vec<crate::globals::InputState>` to `InputClusterState`
  in `src/state.rs` (`#[derive(Default)]` still covers the struct — empty Vec).
- Repointed the two accessors in `globals.rs`: `input_stack(k)` /
  `set_input_stack(k, v)` now index the `Vec` via `with_state`/
  `with_state_mut` (no `unsafe`). Both keep their `InputState`-valued
  signatures (the bridge is unchanged).
- Allocation (`ini.rs`): replaced
  `tre_input_stack = xmalloc_array::<InputState>(tre_stack_size)` with a
  `with_state_mut` block writing `vec![<zero InputState>; stack_size + 1]`
  (matching the former `n+1` shape; `InputState` has no `Default`, so the zero
  record is written out field-by-field).
- Cleanup (`tt_cleanup`): dropped `input_stack` (`= Vec::new()`) folded into
  the existing input cleanup block instead of `libc::free`.
- Removed both `#[link_name = "input_stack"]` alias decls (alloc + cleanup),
  the raw extern in `globals.rs`, and the C def in `xetex-ini.c`.

**Newtype debt (§4 check):** element type is `input_state_t`, a struct
typedef already represented as the Rust `InputState` struct — no type info is
lost, so **no** worklist row (checked, none needed). Contrast with the prior
sessions' scalar typedefs (`UnicodeScalar`, `str_number`).

**longjmp / Drop (§7, handled):** identical to sessions 11–15 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort never
jumps over the `Vec`; `Drop` doesn't run on abort (matches prior C).
`tt_cleanup` is the normal teardown and now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 298 → 297.
- Raw `unsafe { ... }` text-search count: 679 → 677.
- `#[link_name]` aliases in `ini.rs`: −2.
- 10 Rust-owned arena arrays total (save_stack, param_stack, the 5 session-13
  line/group stacks, eof_seen, buffer, + input_stack).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `input_stack` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Two input arrays remain: `input_file` (`UFILE ** == *mut *mut c_void`) and
  `nest` (`list_state_record * == *mut ListStateRecord`).
  - `input_file` is the more involved of the two despite its plain-pointer
    element: besides `input_file(k)`/`set_input_file(k, v)` it has a
    pointer-escaping accessor `input_file_ptr(k)` (returns
    `&input_file[k]` as `*mut *mut UFile` for `u_open_in`) — handle it like
    `buffer_ptr` (obtain via `as_mut_ptr().add(k)` past the borrow; the
    thread-local owner never reallocates). Stored as `Vec<*mut c_void>`. Sized
    `max_in_open + 1`. Not in format dump.
  - `nest` (`Vec<ListStateRecord>`) has ~12 field-projection accessors
    (`nest_mode`, `nest_tail`, `set_nest_tail`, `nest_to_cur_list`,
    `cur_list_to_nest`, …) — more accessors to repoint but no pointer escape.
    `ListStateRecord` already exists; check it derives/Copy for the zero-init
    `vec!`. Sized `nest_size + 1`. Not in format dump. `grp_stack`-style
    fields may carry semantic typedefs — check per-field, though as a struct
    element the array itself is no debt (same as input_stack here).
- After these two, all `input`-cluster arrays are Rust-owned. Then `strings`
  (`str_start`/`str_pool`) — but those **are** in the format dump/undump, so
  that slice must reconcile the load/dump paths.

## Idiom session 15 — 2026-05-23 — buffer array → Rust-owned Vec

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** move the input `buffer` (`UnicodeScalar * == int32_t *`) into a
Rust-owned `Vec<i32>` on `EngineState.input`. One array, one commit.

**Why buffer now:** of the four remaining input arrays it is the lowest-
difficulty — plain `i32` element type (so the `InputClusterState` still
derives `Default`, no hand-written impl needed, unlike `input_file`/
`input_stack`/`nest`), the only C reference is its `xetex-ini.c`
definition, and it is **not** serialized in the format dump/undump. It is
also fixed-size: `buf_size` is a constant (200 000 in initex) and overflow
paths *error* (`overflow(c"buffer size", …)` in `input.rs`/`ini.rs`)
rather than reallocating, so the "Vec must never realloc during a run"
invariant holds.

**Pre-move grep checks (§11 step 2):** (a) declared `extern UnicodeScalar
*buffer;` in `xetex-xetexd.h:392` — `UnicodeScalar` is a semantic typedef
(newtype debt, see below). (b) two `link_name = "buffer"` aliases in
`ini.rs` (alloc + cleanup) → ownership slice. (c) only `xetex-ini.c`
references it. (d) no foreign-module `extern`/`static mut` redeclaration
(the only hits were the two `ini.rs` aliases being removed).

**Work done:**
- Added `buffer: Vec<i32>` to `InputClusterState` in `src/state.rs`
  (`#[derive(Default)]` still covers the struct).
- Repointed the three accessors in `globals.rs`: `buffer_entry(k)` /
  `set_buffer_entry(k, v)` now index the `Vec` via `with_state`/
  `with_state_mut` (no `unsafe`). `buffer_ptr(offset)` (still `unsafe fn`,
  returns `*mut i32` for the io.rs byte-decode path) obtains the pointer
  via `s.input.buffer.as_mut_ptr().add(offset)` — valid past the borrow
  because the `Vec` is owned by the `'static` thread-local and never
  reallocates/reassigns during a run (same stability the old `raw::buffer`
  pointer had). Only callers are io.rs:390 / io.rs:485.
- Allocation (`ini.rs`): replaced `tre_buffer = xmalloc_array::<i32>(buf_size)`
  with a `with_state_mut` block writing `vec![0i32; buf_size + 1]`
  (matching the former `n+1` shape). Removed the now-redundant explicit
  `write_bytes(tre_buffer, 0, buf_size)` re-zero — the `Vec` is already
  zero-initialized at alloc and nothing reads it before that point.
- Cleanup (`tt_cleanup`): dropped `buffer` (`= Vec::new()`) in the input
  cleanup block instead of `libc::free`.
- Removed both `#[link_name = "buffer"]` alias decls (alloc + cleanup),
  the raw extern in `globals.rs`, and the C def in `xetex-ini.c`.

**Newtype debt (§4 check):** `buffer`'s element type is `UnicodeScalar`,
an `int32_t` typedef for a Unicode scalar value — a semantic type. Added a
worklist row (`buffer[]` → `UnicodeScalar`) in `01a-global-inventory.md`.

**longjmp / Drop (§7, handled):** identical to sessions 11–14 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort
never jumps over the `Vec`; `Drop` doesn't run on abort (matches prior C).
`tt_cleanup` is the normal teardown and now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 299 → 298.
- Raw `unsafe { ... }` text-search count: 681 → 679.
- `#[link_name]` aliases in `ini.rs`: −2.
- 9 Rust-owned arena arrays total (save_stack, param_stack, the 5 session-13
  line/group stacks, eof_seen, + buffer).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `buffer` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- Three input arrays with non-trivial element types remain: `input_file`
  (`*mut *mut c_void`), `input_stack` (`*mut InputState`), `nest`
  (`*mut ListStateRecord`). Each needs the sub-struct to keep deriving
  `Default` — `Vec<T>` derives `Default` for any `T` (empty vec), so no
  hand-written impl is actually required as long as the field type is
  `Vec<…>`; the element types just need to exist in the chosen Rust form
  (`InputState`/`ListStateRecord` already do; `input_file` was a
  `*mut *mut c_void` array of `UFILE*` handles — store as `Vec<*mut
  c_void>` or a typed handle). None is in the format dump. Each still has
  **two** alias decls (alloc + cleanup) + raw extern + C def.
- Alternatively `strings` (`str_start`/`str_pool`) — but those **are** in
  the format dump/undump, so that slice must reconcile the load/dump paths.

## Idiom session 14 — 2026-05-23 — eof_seen array → Rust-owned Vec (last simple input array)

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** finish the simple input-cluster arrays by moving the last one,
`eof_seen` (`*mut bool`), into a Rust-owned `Vec<bool>` on
`EngineState.input`. One array, one commit.

**Why eof_seen now:** it is the last input array with a plain element
type and the exact session-13 shape — sized `max_in_open`, allocated in
the same `ini.rs` block, two `#[link_name]` aliases (alloc `tre_eof_seen`
+ cleanup `tt_cleanup_eof_seen`), the only C reference is its definition
in `xetex-ini.c`, and it is not serialized in the format dump/undump. The
remaining input arrays have harder element types (`input_file`
`*mut *mut c_void`, `input_stack` `*mut InputState`, `nest`
`*mut ListStateRecord`, `buffer` `*mut i32`) and are left for follow-up.

**Pre-move grep checks (§11 step 2):** (a) `xetex-xetexd.h` declares it
`extern bool *eof_seen;` — plain `bool`, no semantic typedef. (b) two
`link_name = "eof_seen"` aliases in `ini.rs` (alloc + cleanup) → ownership
slice. (c) only `xetex-ini.c` references it. (d) no foreign-module
`extern`/`static mut` redeclaration (the only hits were the two `ini.rs`
aliases being removed).

**Work done:**
- Added `eof_seen: Vec<bool>` to `InputClusterState` in `src/state.rs`
  (`#[derive(Default)]` covers it).
- Repointed the two accessors in `globals.rs` (`eof_seen(k)` /
  `set_eof_seen(k, v)`) to index the `Vec` via `with_state`/
  `with_state_mut`. No `unsafe` remains in them.
- Allocation (`ini.rs`): removed the `tre_eof_seen = xmalloc_array::<bool>`
  call and added `s.input.eof_seen = vec![false; n]` to the existing
  session-13 `with_state_mut` block (`n = max_in_open + 1`). C left this
  uninitialized; Rust zero(false)-inits — parity-safe (each slot written
  before read).
- Cleanup (`tt_cleanup`): dropped `eof_seen` (`= Vec::new()`) in the same
  block as the other input arrays instead of `libc::free`.
- Removed both `#[link_name = "eof_seen"]` alias decls (alloc + cleanup),
  the raw extern in `globals.rs`, and the C def in `xetex-ini.c`.

**Newtype debt (§4 check):** none — `eof_seen` is plain `bool *`. Checked,
no worklist row needed.

**longjmp / Drop (§7, handled):** identical to sessions 11–13 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort
never jumps over the `Vec`; `Drop` doesn't run on abort (matches prior C).
`tt_cleanup` is the normal teardown and now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 300 → 299.
- Raw `unsafe { ... }` text-search count: 683 → 681.
- `#[link_name]` aliases in `ini.rs`: −2.
- 8 Rust-owned arena arrays total (save_stack, param_stack, the 5 session-13
  line/group stacks, + eof_seen). All simple input-cluster arrays now done.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of `eof_seen` in `xetex/*.c` (minus macos): empty.

**For the next session:**
- The simple input arrays are exhausted. Next ownership-bearing input
  arrays have non-trivial element types and need a hand-written
  `impl Default` if the sub-struct can no longer derive it: `input_file`
  (`*mut *mut c_void`), `input_stack` (`*mut InputState`), `nest`
  (`*mut ListStateRecord`), `buffer` (`*mut i32`). None in the dump.
  Each still has **two** alias decls (alloc + cleanup) + raw extern + C def.
- Alternatively `strings` (`str_start`/`str_pool`) — but those **are** in
  the format dump/undump, so that slice must reconcile the load/dump paths.

## Idiom session 13 — 2026-05-23 — input line/group stacks → Rust-owned Vecs (5 arrays)

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** extend the session-12 input-cluster arena pattern to the
homogeneous `*mut i32` line/group stacks: `line_stack`, `grp_stack`,
`if_stack`, `source_filename_stack`, `full_source_filename_stack`. Five
arrays, one commit.

**Why these five together:** identical `*mut i32` element type, all sized
`max_in_open`, allocated in the same `ini.rs` block, the only C reference
is each one's definition in `xetex-ini.c`, and none is serialized in the
format dump/undump (same no-dump property as `param_stack`/`save_stack`).
`eof_seen` (`*mut bool`) is left for a later slice (different element
type); the trickier element types (`input_file`, `input_stack`, `nest`,
`buffer`) remain too.

**Work done:**
- Added five `Vec<i32>` fields to `InputClusterState` in `src/state.rs`
  (`line_stack`, `grp_stack`, `if_stack`, `source_filename_stack`,
  `full_source_filename_stack`; `#[derive(Default)]` covers them).
- Repointed all accessors in `globals.rs` (getters
  `line_stack`/`grp_stack`/`if_stack`/`source_filename_stack`/
  `full_source_filename_stack`, setters `set_*`) to index the `Vec`s via
  `with_state`/`with_state_mut`. No `unsafe` remains in them.
- Allocation (`src/ini.rs`): replaced the five `xmalloc_array::<i32>` calls
  with one `with_state_mut` block writing `vec![0i32; max_in_open + 1]`
  each (matching the former `n+1` shape). C left these uninitialized;
  Rust zero-inits, parity-safe (each slot is written before read).
- The four explicit init writes in `ini.rs` (`*tre_source_filename_stack
  .add(0) = 0` etc., plus `if_stack[0] = TEX_NULL`) were rewritten to the
  new `set_*` accessors.
- Cleanup (`tt_cleanup`): dropped the five arrays (`= Vec::new()`) instead
  of `libc::free`.
- Removed both `#[link_name]` alias decls per array (alloc `tre_*` +
  cleanup `tt_cleanup_*` = 10 decls), the five raw externs in `globals.rs`,
  and the five C defs in `xetex/xetex-ini.c` (3 in the line-stack block,
  2 in the grp/if block). `eof_seen` stays a C global.

**Newtype debt (§4 check):** `grp_stack` is `save_pointer *` and
`source_filename_stack`/`full_source_filename_stack` are `str_number *` —
three worklist rows added to `01a-global-inventory.md`. `line_stack`
(`int32_t *`, line numbers) and `if_stack` (`int32_t *`, `cond_ptr`
halfwords) are plain `int32_t` — no typedef-loss row (the `if_stack`
elements become a `MemPtr` in the later pointer sweep, not a typedef row).

**longjmp / Drop (§7, handled):** identical to sessions 11–12 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort
never jumps over the `Vec` storage; `Drop` simply doesn't run on abort
(matches prior C, which never freed on abort). `tt_cleanup` is the normal
teardown and now drops the `Vec`s.

**Metrics after this session:**
- `globals.rs` raw extern globals: 305 → 300.
- Raw `unsafe { ... }` text-search count: 693 → 683.
- `#[link_name]` aliases in `ini.rs`: −10.
- 6 Rust-owned arena arrays total (save_stack, param_stack + these 5… = 7).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings only.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep` of the 5 symbols in `xetex/*.c` (minus macos): empty.

**Surprises / gotchas:**
- Unlike `param_stack`, these arrays have explicit slot-0 init writes in
  `ini.rs` (`source_filename_stack[0]=0`, `grp_stack[0]=0`,
  `if_stack[0]=TEX_NULL`, etc.) that used the raw `tre_*` pointers — those
  had to be rewritten to the new `set_*` accessors, not just deleted.
- The five arrays span **two** raw-extern blocks and **two** C-def blocks
  in `xetex-ini.c` (line/filename block + grp/if block); grep each.

**For the next session:**
- `eof_seen` (`*mut bool`) is the last simple input array — same shape,
  `vec![false; max_in_open + 1]`. Then the harder element types:
  `input_file` (`*mut *mut c_void`), `input_stack` (`*mut InputState`),
  `nest` (`*mut ListStateRecord`), `buffer` (`*mut i32`). None in the dump.
- Two alias decls per array (alloc + cleanup) + raw extern + C def, same
  commit, every time.

## Idiom session 12 — 2026-05-23 — param_stack array → Rust-owned Vec (first input-cluster ownership move)

**Type:** execution (workstream 01, ownership-bearing slice).

**Goal:** extend the session-11 arena-ownership pattern into the `input`
cluster, starting with its smallest, cleanest array: `param_stack` (the
macro-parameter stack). One array, one commit.

**Why param_stack first (of the input arrays):** plain `*mut i32`
element type (no `InputState`/`ListStateRecord` complications), a single
allocation site (`ini.rs:3256`), a single `tt_cleanup` free, exactly two
`#[link_name]` aliases (alloc `tre_param_stack`, cleanup
`tt_cleanup_param_stack`), the only C reference is its definition in
`xetex-ini.c`, and it is **not** serialized in the format dump/undump —
so there is no load/dump path to reconcile (same property as
save_stack). The other input arrays (`input_stack`, `input_file`,
`line_stack`, `eof_seen`, `grp_stack`, `if_stack`,
`source_filename_stack`, `full_source_filename_stack`, `nest`, `buffer`)
remain for follow-up slices.

**Work done:**
- Added a new `InputClusterState` sub-struct to `EngineState`
  (`pub input`), holding `param_stack: Vec<i32>` (`#[derive(Default)]`;
  `Vec` defaults empty). Named to avoid colliding with the existing
  `crate::globals::InputState` per-input-record type.
- Repointed the two `param_stack` accessors in `globals.rs`
  (`param_stack(k)` / `set_param_stack(k, v)`) to index the `Vec` via
  `state::with_state` / `with_state_mut`. The index is safe and
  bounds-checked; no `unsafe` remains in these accessors.
- Allocation (`src/ini.rs:3256`): replaced
  `tre_param_stack = xmalloc_array::<i32>(tre_param_size)` with a
  `with_state_mut` writing `vec![0i32; (param_size as usize) + 1]`
  (matching the old `n+1` element shape). C left the slot uninitialized;
  Rust zero-inits, parity-safe because TeX writes each param-stack slot
  before reading it.
- Cleanup (`tt_cleanup`): removed `libc::free(tt_cleanup_param_stack)`
  and instead drops the `Vec` (`= Vec::new()`).
- Removed both `#[link_name = "param_stack"]` alias decls (the alloc
  `tre_param_stack` and the cleanup `tt_cleanup_param_stack`), the
  `globals.rs` raw extern, and the `int32_t *param_stack;` C definition
  in `xetex/xetex-ini.c`. The scalars `param_ptr`, `max_param_stack`,
  `param_size` stay C globals (only the array moved).

**Newtype debt (§4 check):** `param_stack`'s declared C type is
`int32_t *` (plain `int32_t` elements, no semantic typedef), so **no**
Deferred-newtype-worklist row is required. Noted here that the check was
done. (The elements are token-list pointers and would become a `MemPtr`
in a later taxonomy sweep, but that follows the arena/pointer work, not a
typedef-loss row.)

**longjmp / Drop (§7, handled):** identical reasoning to session 11 —
`EngineState` lives in the `'static` thread-local, so a `longjmp` abort
never jumps over the `Vec`'s storage (no use-after-free); `Drop` simply
does not run on abort, matching the prior C behavior (the array was never
freed on abort either). `tt_cleanup` remains the normal-path teardown and
now drops the `Vec`.

**Metrics after this session:**
- `globals.rs` raw extern globals: 306 → 305.
- Raw `unsafe { ... }` text-search count: 695 → 693.
- `#[link_name]` aliases in `ini.rs`: −2.
- Second Rust-owned arena array; first in the `input` cluster.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, pre-existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.
- `grep param_stack xetex/*.c | grep -v macos`: empty (no leftover C ref).

**Surprises / gotchas:**
- The input arrays share **one** allocation block (`ini.rs:3248-3256`),
  but each has **two** extern-alias declarations in separate blocks: the
  `tre_*` block (alloc path, ~line 1819+) and the `tt_cleanup_*` block
  (free path, ~line 3570+). Both must be removed per array. (save_stack
  had only one alias pair; this is the pattern for the input arrays.)
- IDE flagged `param_stack` as an "unused import" in `scan.rs` mid-edit —
  stale; the accessor still exists and `cargo check` is clean.

**For the next session:**
- Continue the input-cluster ownership moves, reusing this exact shape.
  Next-easiest are the other `*mut i32` arrays sized by `max_in_open`
  (`line_stack`, `grp_stack`, `if_stack`, `source_filename_stack`,
  `full_source_filename_stack`) and `*mut bool` `eof_seen` — same
  no-dump property, plain element types. `input_file`
  (`*mut *mut c_void`) and `input_stack` (`*mut InputState`) and `nest`
  (`*mut ListStateRecord`) need element-type care (Copy/Default). None of
  the input arrays are in the format dump.
- Remember: two alias decls per array (alloc `tre_*` + cleanup
  `tt_cleanup_*`), plus the raw extern and the C def — all four edits in
  the same commit.

## Idiom session 11 — 2026-05-23 — save_stack arena → Rust-owned Vec (first ownership move)

**Type:** execution (workstream 01, first ownership-bearing slice).

**Goal:** move the `save_stack` array out of C `malloc`/`free` ownership
into a Rust-owned `Vec<MemoryWord>` on `EngineState`, establishing the
arena-ownership pattern (§5/§7) on the smallest self-contained allocated
array before tackling `strings`/`eqtb`/`fonts`.

**Why save_stack first:** single fixed-capacity array (`save_size + 1`
words), one allocation site, one `tt_cleanup` free, two `#[link_name]`
aliases, and — crucially — **not** serialized in the format dump/undump
(the save stack is empty at `\dump`), so there is no load/dump path to
reconcile.

**Work done:**
- Added `data: Vec<MemoryWord>` to `SaveStackState` (`#[derive(Default)]`
  still holds: `Vec` defaults to empty).
- Repointed the 10 `save_stack_*` accessors in `globals.rs` to index the
  `Vec` via `state::with_state`. The only remaining `unsafe` is the
  `MemoryWord` union projection (per §5); the `Vec` index is safe and
  bounds-checked.
- Allocation (`src/ini.rs`): replaced the
  `xmalloc_array::<MemoryWord>(save_size)` call with a zeroed
  `vec![MemoryWord{..0}; save_size + 1]` written into `EngineState`
  (matching the old `n+1` element shape). C `xmalloc_array` left
  uninitialized; Rust zero-inits, which is parity-safe because TeX writes
  every save-stack slot before reading it.
- Cleanup (`tt_cleanup`): removed `libc::free(save_stack)` and instead
  drops the `Vec` (`= Vec::new()`), releasing the arena between runs.
- Removed both `#[link_name = "save_stack"]` aliases (the allocation
  alias and the cleanup-free alias), the `globals.rs` raw extern, and the
  `memory_word *save_stack;` C definition. `save_size` stays a C global
  (still aliased/used) — only the array moved.

**longjmp / Drop (the §7 hazard, handled):** `EngineState` lives in the
`'static` thread-local, so a `longjmp` on the abort path does not jump
over the `Vec`'s storage (no use-after-free); its `Drop` simply does not
run on abort, which matches the pre-existing behavior (the C array was
also never freed on abort). `tt_cleanup` remains the explicit teardown
on the normal path and now drops the `Vec` instead of `free`-ing a C
pointer.

**Metrics after this session:**
- `globals.rs` raw extern globals: 307 → 306.
- Raw `unsafe { ... }` text-search count: 701 → 695.
- `#[link_name]` aliases in `ini.rs`: −2.
- First Rust-owned arena array (no scalar-count change).

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2 (exercises save/restore
  heavily — the key check for this slice).
- `cargo test -p tectonic --test engine_rust`: 7/7 (incl. save_restore).
- `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- `xmalloc_array(n)` allocates `n + 1` elements (`src/ini.rs:1746`); the
  `Vec` must match that inclusive size or off-by-one indexing breaks.
- The save-stack arena is allocated unconditionally in the common engine
  init block (one site), not split across initex/format-load paths like
  `mem`, which simplified the move.
- `MemoryWord` is a `Copy` union with no `Default`; build the zero word
  explicitly as `MemoryWord { b32: B32x2 { s0: 0, s1: 0 } }` for `vec!`.

**For the next session:**
- The arena-ownership pattern is now proven. Next ownership targets, in
  rough difficulty order: the `input` cluster arrays (`input_stack`,
  `nest`, `param_stack`, the per-`max_in_open` stacks — several arrays,
  same no-dump property as save_stack), then `strings`
  (`str_pool`/`str_start`, which **are** in format dump/undump), then
  `fonts` arrays, then the big `mem`/`eqtb` arenas (§5).
- Reuse exactly this shape: `Vec` field, accessors index it, allocation
  writes the `Vec`, `tt_cleanup` drops it, remove aliases + raw extern +
  C def in the same commit.

## Idiom session 10 — 2026-05-23 — eqtb scalar set Phase-B bridge

**Type:** execution (workstream 01).

**Goal:** migrate the small clean `eqtb` scalar set, keeping the
ownership/alias-bearing members (`eqtb`, `hash`, `prim` arrays,
`hash_extra`, `eqtb_top`, `hash_top`) out of scope.

**Work done:**
- Added `EqtbState` under `EngineState` and moved five scalars into it:
  `hash_used`, `hash_high`, `no_new_control_sequence`, `prim_used`,
  `cs_count`.
- Repointed the `globals.rs` accessors at `state::with_state` /
  `with_state_mut`; removed the raw externs (leaving `hash`,
  `hash_extra`, `prim`, `eqtb_top`) and the C definitions in
  `xetex/xetex-ini.c` (leaving `hash`, `hash_extra`, `hash_top`,
  `eqtb_top`, `prim`).
- No newtype-worklist rows: all five are plain `int32_t`/`bool` with no
  semantic C typedef.

**Metrics after this session:**
- `globals.rs` raw extern globals: 312 → 307.
- Raw `unsafe { ... }` text-search count: 711 → 701.
- `globals.rs`: 77 globals now state-backed.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- `hash_high`, `cs_count`, and `no_new_control_sequence` are read inside
  `id_lookup` (`src/hash.rs`), but only on the cs-not-found / interning
  branch — a defined control sequence returns at the "found" branch
  without touching them. So in steady state they are creation-time /
  cache-miss reads, not per-token hot like the deferred `cur_chr` /
  `cur_cmd` tokenizer scalars. Bridging them is acceptable.
- `hash_extra`/`eqtb_top` (which carry `#[link_name]` init aliases) and
  the `hash`/`prim`/`eqtb` arrays are interleaved with these scalars in
  both files and were deliberately left for the ownership-aware `eqtb`
  array slice.
- Verified (grep) no non-macOS C reader and no separate `extern`
  redeclaration in other Rust modules before moving.

**For the next session:**
- Remaining clean scalar wins are getting sparse: a few more non-hot
  `scanner` scalars (e.g. `shown_mode`, `mag_set`, `radix`,
  `expand_depth_count`) and `mem` bookkeeping scalars
  (`var_used`, `dyn_used`, `lo_mem_max`, `hi_mem_min`, `avail`, `rover`,
  `mem_end`) — but the `mem` set is tightly coupled to the arena and is
  better tackled as part of the `Mem` arena move (§5).
- The big remaining work is the ownership-bearing clusters (`strings`,
  `fonts` arrays, `shipout` file-handles, the `save_stack`/`save_size`
  pointer pair, the `eqtb`/`hash`/`prim` arrays) — each needs a single
  ownership-aware slice that also fixes `tt_cleanup`/`#[link_name]`.

## Idiom session 9 — 2026-05-23 — save_stack scalar cluster Phase-B bridge

**Type:** execution (workstream 01).

**Goal:** migrate the `save_stack` scalar cluster, keeping the
ownership-bearing members (`save_stack` pointer, `save_size` capacity)
out of scope so this stays a clean no-cleanup-alias slice.

**Work done:**
- Added `SaveStackState` under `EngineState` and moved the five
  save-stack scalars into it: `save_ptr`, `max_save_stack`, `cur_level`,
  `cur_group`, `cur_boundary`.
- Repointed the `globals.rs` accessors at `state::with_state` /
  `with_state_mut`; removed the raw externs (leaving `save_stack` and
  `save_size`) and the C definitions in `xetex/xetex-ini.c`.
- Recorded the `cur_group` → `GroupCode` newtype debt in the worklist
  (`01a-global-inventory.md`); the others are plain int/uint counts.

**Metrics after this session:**
- `globals.rs` raw extern globals: 317 → 312.
- Raw `unsafe { ... }` text-search count: 721 → 711.
- `globals.rs`: 72 globals now state-backed.

**Validation:**
- `cargo check -p tectonic_engine_xetex`: pass, existing warnings.
- `cargo test -p tectonic --test tex-outputs`: 29/29.
- `cargo test -p tectonic --test trip`: 2/2.
- `cargo test -p tectonic --test engine_rust`: 7/7.
- `cargo test -p tectonic --test latex_goldens`: 4/4.

**Surprises / gotchas:**
- `save_size` (capacity, set at init) and the `save_stack` pointer carry
  ownership/`#[link_name]`/cleanup edges and were deliberately left for
  a later ownership-aware slice; they sit interleaved with the scalars
  in both `globals.rs` and `xetex-ini.c`, so each scalar was removed
  individually around them.
- `cur_level`/`cur_group`/`cur_boundary` and `save_ptr` are touched on
  every group entry/exit (`new_save_level`/`unsave`), but that is not a
  per-token inner loop, so the thread-local bridge cost is acceptable
  here (unlike the tokenizer scalars deferred in session 8).
- Verified (grep) no non-macOS C reader and no separate `extern`
  redeclaration in other Rust modules before moving.

**For the next session:**
- Remaining clean scalar wins: small `eqtb` scalar set (`hash_used`,
  `hash_high`, `no_new_control_sequence`, `prim_used`, `cs_count`) and
  more non-hot `scanner` scalars. Then the ownership-bearing clusters
  (`strings`, `fonts` arrays, `shipout` file-handles, and the
  `save_stack`/`save_size` pointer pair) each need a single
  ownership-aware slice that also fixes `tt_cleanup`/`#[link_name]`.
- Keep `with_state` closures single-field; do not call translated engine
  code while a borrow is live.

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
