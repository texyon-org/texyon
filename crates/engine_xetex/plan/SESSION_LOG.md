# Session log

Append-only log of what each session did. The newest entry goes at the
**top**, immediately under this header. Each session follows this
template.

## Entry template

```
## Session N — YYYY-MM-DD — <short title>

**Goal:** one-sentence description of what this session planned to do.

**Starting state:**
- Parity: X/Y
- Streak: N

**Work done:**
- <bullet list of concrete actions: functions translated, design decisions,
  docs updated. Include commit hashes.>

**Ending state:**
- Parity: X/Y (delta: +/-N)
- Streak: N

**Surprises / gotchas:**
- <anything that wasted time, or that the next session should know>

**For the next session:**
- <handoff: what's the obvious next piece of work, any half-done state>
```

---

## Session 17 — 2026-04-18 — Phase 11: scattered xetex-xetex0.c functions (13 functions)

**Goal:** continue translating from xetex-xetex0.c and finish xetex-io.c's open_or_close_in.

**Starting state:**
- Parity: 29/29
- Streak: 16

**Work done:**
- Translated `open_or_close_in` (xetex-io.c:294) into `src/io.rs`. Added globals:
  `cur_chr`, `cur_val`, `cur_name/area/ext`, `name_in_progress`, `stop_at_space`,
  `read_open[17]`, `read_file[16]`. (f4a94ffc)
- Translated `delete_token_ref` (xetex-xetex0.c:976) and `delete_glue_ref` (984)
  into `src/mem.rs`. (3e1f337e)
- Translated `begin_diagnostic` (1785) and `end_diagnostic` (1797) into `src/output.rs`.
  Added global `old_setting`, constants HISTORY_SPOTLESS/WARNING_ISSUED/TRACING_ONLINE. (1be1cb50)
- Translated `print_mode` (1327) and `print_in_mode` (1356) into `src/output.rs`. (d30eb43b)
- Translated `print_group` (3464) into `src/output.rs`. Added globals: `save_stack`,
  `save_ptr`, `max_save_stack`, `save_size`, `cur_level`, `cur_group`. Added all group-code
  constants, save-stack b16/b32 accessors. (6a81482c)
- Created `src/save.rs`. Translated `new_save_level` (3859), `eq_save` (3914),
  `eq_word_define` (3954), `geq_word_define` (3977), `save_for_after` (3984). Added globals:
  `cur_boundary`, `_xeq_level_array`. Added eqtb word/b16s0/b16s1 accessors, xeq_level
  read/write, set_save_stack_word. (5c29bdaf)
- Translated `int_error` (15, was static) into `src/errors.rs`. Added prototype to
  xetex-xetexd.h. (9840db6e)
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 17
- 116 Rust functions total; 13 new this session.
- New module `src/save.rs` created.

**Surprises / gotchas:**
- `read_file[16]` in globals.rs declared as `[*mut c_void; 16]` to avoid circular type
  dependency with io.rs (which defines UFile). Cast to `*mut *mut io::UFile` in accessor.
- `_xeq_level_array` is 1114730 elements — declared as fixed-size extern array. Access uses
  `p - INT_BASE` as index (INT_BASE = 7826729).
- save-stack writes to b16.s1, b16.s0, and b32.s1 on same MemoryWord slot are non-overlapping
  (bytes 2-3, 0-1, and 4-7 respectively in little-endian layout). Safe to do in sequence.
- `errors.rs` already had `print_cstr/print_int/print_char` in its `extern "C"` block;
  don't add a duplicate extern block.

**For the next session:**
- `prepare_mag` (4017) is now unblocked (int_error is Rust). Needs `ttstub_diag_printf`
  (variadic — declare in Rust as `fn ttstub_diag_printf(d: *mut c_void, fmt: *const c_char, ...)`).
- `eq_define` (3936) and `geq_define` (3968) need `eq_destroy` (complex switch, calls
  delete_token_ref (Rust), delete_glue_ref (Rust), flush_node_list (C), delete_sa_ref (C)).
- `push_nest`/`pop_nest` (1385/1401) need `nest` and `cur_list` globals (list_state_record
  struct array). Moderate complexity.
- `unsave` (3999) is complex (goto→loop, many save-stack variants).
- Consider `group_warning` (3688) and `if_warning` (3725) — short diagnostic functions.

---

## Session 16 — 2026-04-18 — Phase 10: xetex-io.c partial (4 functions)

**Goal:** translate as many functions from `xetex-io.c` as feasible.

**Starting state:**
- Parity: 29/29
- Streak: 15

**Work done:**
- Created `src/io.rs` with `UFile` struct (`#[repr(C)]`, 32-byte layout on x86_64),
  encoding constants, and all extern declarations.
- Added `name_of_file`, `name_length`, `name_of_file16`, `name_length16` accessors to `globals.rs`.
- Moved C file-statics `name_of_input_file` and `abspath_of_input_file` to Rust
  `#[no_mangle] pub static mut` in `src/io.rs`.
- Translated `tt_xetex_open_input` (line 22), `u_open_in` (line 127),
  `make_utf16_name` (line 481), `get_uni_c` (line 373). (979cde6d)
- `u_close` left in C — `ucnv_close` (ICU) produces `undefined symbol` when
  referenced from Rust; would need `cargo:rustc-link-lib=icuuc` in build.rs.
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 16

**Surprises / gotchas:**
- `ucnv_close` (ICU) causes a linker error when called from Rust even though
  the C code in the same file also calls it. The ICU library is linked via a
  DEP_ mechanism that the C build knows about but that Rust's `extern "C"`
  blocks don't pull in automatically. Workaround: keep `u_close` in C.
  Future fix: add `cargo:rustc-link-lib=icuuc` to `build.rs` if ICU is available.
- `get_uni_c`'s `goto bad_utf8` inside a nested switch: the goto jumps INTO the
  switch, right before `case 5:` and `case 4:`. Translated by checking
  `extra_bytes >= 4` before the loop (handles cases 4/5) and using early-return
  with `ttstub_input_ungetc` for bad continuation bytes.
- `make_utf16_name`'s static local `name16len` → Rust `static mut NAME16LEN`.
- `u_open_in`'s unused `fopen_mode` parameter → `_fopen_mode` in Rust.

**For the next session:**
- `xetex-io.c` remaining 4 C functions: `u_close` (ICU blocker), 
  `set_input_file_encoding` (ICU), `input_line` (complex — ICU + TECkit),
  `open_or_close_in` (calls many C functions).
  The complex ones can be deferred long-term; consider `open_or_close_in` next.
- Or switch to `xetex-xetex0.c` for more yield.

---

## Session 15 — 2026-04-18 — Phase 9: xetex-texmfmp.c fully translated

**Goal:** translate all remaining functions from `xetex-texmfmp.c`.

**Starting state:**
- Parity: 29/29
- Streak: 14

**Work done:**
- Added `ttstub_input_open/get_size/get_mtime/close/seek/read` externs, `TTBC_FILE_FORMAT_TEX`, `INVALID_HANDLE` constant, and `START_TIME_STR` Rust static to `src/texmfmp.rs`.
- Commit 23d0822a: `make_src_special` (line 274), `getfilesize` (line 155), `getfiledump` (line 189), `compare_paths` (line 243, was static), `is_new_source` (line 257), `remember_source_info` (line 264). C statics `last_source_name`/`last_lineno` moved to Rust `LAST_SOURCE_NAME`/`LAST_LINENO`.
- Commit 13fb941d: `makepdftime` (line 30, was static), `init_start_time` (line 84), `getcreationdate` (line 91), `getfilemoddate` (line 120). C static `start_time_str[30]` moved to Rust `START_TIME_STR`. `xetex-texmfmp.c` is now DONE.
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 15
- `xetex-texmfmp.c`: DONE (5th file fully translated)

**Surprises / gotchas:**
- C's `getfilemoddate` passed a 20-byte `buf` to `makepdftime` which uses `TIME_STR_SIZE=30`. This is an overflow bug in C. Rust translation uses 30-byte buffer throughout, fixing it silently.
- `is_new_source` and `make_src_special` both leaked the `gettexstring` result in C. Added `libc::free` in Rust.
- `makepdftime`'s UTC-offset format `%+03d` maps exactly to Rust `{:+03}`.
- `compare_paths` with null `last_source_name` would crash on the first call before `remember_source_info`. Added null guard in Rust (treats null as empty string).

**For the next session:**
- `xetex-texmfmp.c` is fully done. Five C files are now empty.
- Next candidates: `xetex-io.c` (555 LOC, ~?? fns) or continue with `xetex-xetex0.c` (still 18,387 LOC, 268 fns).
- `xetex-io.c` contains the UTF-8 lookup tables (`bytesFromUTF8`, etc.) and I/O wrappers. Likely a mix of straightforward and complex functions.

---

## Session 14 — 2026-04-18 — Phase 8: gettexstring, maketexstring, getmd5sum

**Goal:** translate `gettexstring`, `maketexstring`, `checkpool_pointer`, `convertStringToHexString`, and `getmd5sum` from `xetex-texmfmp.c`.

**Starting state:**
- Parity: 29/29
- Streak: 13

**Work done:**
- Fixed missing SESSION_LOG.md entries for sessions 12 and 13 (reconstructed from git log / MODULE_STATUS.md).
- Added extern declarations for C lookup tables `firstByteMark`, `bytesFromUTF8`, `offsetsFromUTF8` (from `xetex-io.c`), `ttstub_get_file_md5`, `ttbc_get_data_md5`, `xmalloc` to `src/texmfmp.rs`.
- Translated `gettexstring` (texmfmp.c:282) — pool UTF-16 → heap UTF-8. C's falling-through switch (backward writes with `--j`) rewritten as forward writes using a saved `start = j` offset. Equivalent byte ordering.
- Translated `checkpool_pointer` (texmfmp.c:237, was static) → private Rust fn `check_pool_pointer`.
- Translated `maketexstring` (texmfmp.c:245) — heap UTF-8 → pool UTF-16. C's falling-through case 5..0 switch translated as `for _ in 0..extra_bytes` loop. Calls `crate::string_pool::make_string()` directly in Rust.
- Translated `convertStringToHexString` (texmfmp.c:396, was static) → private Rust fn.
- Translated `getmd5sum` (texmfmp.c:413) — MD5 of file/string, hex-encoded into pool. Calls Rust `gettexstring` directly.
- Deleted all 5 C bodies. Committed as 717cf3d4.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 14

**Surprises / gotchas:**
- `gettexstring`: The C switch writes bytes backward using `--j` pre-decrement then restores `j += bytesToWrite`. In Rust, saving `start = j` before `j += bytes_to_write` and writing forward gives identical results.
- `maketexstring`: The C `switch (extraBytes)` with fallthrough from case N down to case 1 (case 0 is empty) reduces to exactly `extra_bytes` identical iterations — a simple `for _ in 0..extra_bytes` loop.
- `make_string()` is already a Rust fn in `crate::string_pool` (`pub unsafe extern "C"`); can be called directly without FFI.
- `getmd5sum` calls `gettexstring(s)` — since both are now Rust, it's a direct crate-internal call.

**For the next session:**
- `xetex-texmfmp.c` still has: `makepdftime` (static, complex timezone math), `init_start_time`, `getcreationdate` (reads C-static `start_time_str`), `getfilemoddate`, `getfilesize`, `getfiledump`, `compare_paths` (static), `is_new_source`, `remember_source_info` (manages C-static `last_source_name`), `make_src_special`.
- Easiest next targets: `make_src_special` (calls `gettexstring`+writes to pool, no statics), and `getfilemoddate`/`getfilesize`/`getfiledump` (call `gettexstring`+bridge file I/O).
- `is_new_source`/`remember_source_info` require handling `last_source_name` (C static `char*`) — defer or un-static it.
- `makepdftime` + `init_start_time` + `getcreationdate` form a cluster; defer (complex timezone `struct tm` math).

---

## Session 13 — 2026-04-18 — Phase 7: get_date_and_time, get_seconds_and_micros

**Goal:** translate the two time-related helpers from `xetex-texmfmp.c`.

**Starting state:**
- Parity: 29/29
- Streak: 12

**Work done:**
- Translated `get_date_and_time` (texmfmp.c:116) and `get_seconds_and_micros` (texmfmp.c:128) into `src/texmfmp.rs`. Both are pure libc calls with no engine globals. (4bafb5ef)
- Updated plan docs. (f8aeb73d)

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 13

**Surprises / gotchas:**
- None; these were the simplest functions in the file.

**For the next session:**
- Continue with `xetex-texmfmp.c`: `gettexstring`, `maketexstring`, `getmd5sum` are the next natural cluster (string-pool UTF-8/UTF-16 conversion + MD5).

---

## Session 12 — 2026-04-18 — Phase 6: tt_xetex_set_int/string_variable

**Goal:** translate `tt_xetex_set_int_variable` and `tt_xetex_set_string_variable` from `xetex-engine-interface.c`.

**Starting state:**
- Parity: 29/29
- Streak: 11

**Work done:**
- Translated `tt_xetex_set_int_variable` (engine-interface.c:25) and `tt_xetex_set_string_variable` (engine-interface.c:44) into `src/engine_interface.rs`. Two functions remain in C permanently: `tt_engine_xetex_main` (setjmp/longjmp) and `__terrible_aarch64_musl_linker_hack`. (73725033)
- Back-filled clippy lints in `output.rs` and `scaled_math.rs`. (73725033)
- Updated plan docs. (f5dd9c29)

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 12

**Surprises / gotchas:**
- `xetex-engine-interface.c` is marked IN PROGRESS — 50%: 2 of 4 functions translated, 2 permanently in C.

**For the next session:**
- Next target: `xetex-texmfmp.c` — start with the pure time helpers (`get_date_and_time`, `get_seconds_and_micros`).

---

## Session 11 — 2026-04-18 — xetex-errors.c fully translated

**Goal:** translate all 7 functions from `xetex-errors.c`.

**Starting state:**
- Parity: 29/29
- Streak: 10

**Work done:**
- Added globals: `interaction`, `history`, `error_count`, `use_err_help`,
  `halt_on_error_p`, `log_opened`, `job_name`, `set_selector`, `help_line`
  getter. (ea79fbfd)
- Created `src/errors.rs` with 5 public + 2 private functions. Deleted C bodies.
  `xetex-errors.c` is now empty. (480fe9f1)
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 11

**Surprises / gotchas:**
- `_tt_abort` called with `"%s", s` in `fatal_error`; simplified to `_tt_abort(s)`
  directly (the only effect of `%s` is to format `s`), avoiding variadic Rust FFI.
- `help_ptr` loop uses local counter written back via `set_help_ptr` after the loop;
  equivalent to C's in-place mutation since no other code reads it mid-loop.
- `overflow` and `confusion` return `!` — `post_error_message` doesn't, but
  `_tt_abort` does, so the `-> !` propagates correctly through the tail call.

**For the next session:**
- 4 C files are now emptied. Next smallest remaining files:
  `xetex-engine-interface.c` (112 LOC), `xetex-texmfmp.c` (467 LOC).
- Consider `xetex-engine-interface.c` first — it is the engine entry point
  and likely has many globals, but is very short.

---

## Session 10 — 2026-04-18 — Phase 1 complete; xetex-stringpool.c translated

**Goal:** translate `m_log` (last blocked Phase 1 function) and all 8 functions
from `xetex-stringpool.c`.

**Starting state:**
- Parity: 29/29
- Streak: 9

**Work done:**
- Added globals: `max_strings`, `init_str_ptr`, `init_pool_ptr`, `buffer_entry`,
  `help_ptr`/`set_help_ptr`/`set_help_line`, `two_to_the`, `spec_log`,
  `set_str_start`, `set_str_ptr`. (a9551603)
- Translated `m_log` in `src/scaled_math.rs`; removed the C body; Phase 1 is
  now 100% complete. (7509cc5d)
- Created `src/string_pool.rs` with all 8 functions; deleted C bodies from
  `xetex-stringpool.c`. (5e966ad6)
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 10

**Surprises / gotchas:**
- `load_pool_strings` in C iterates a static `string_constants[]` array;
  Rust translates this as a hardcoded `&[&[u8]]` slice — matches exactly.
- `str_eq_buf` had an if/else-if/else where both first branches return false;
  collapsed to `if hi_fail || lo_fail { return false; } j += 1;` for clippy.
- `help_ptr()` getter is currently unused (only setter used by m_log); added
  `#[allow(dead_code)]` pending future error-handling translations.

**For the next session:**
- Three C files are now emptied: xetex-scaledmath.c, xetex-output.c,
  xetex-stringpool.c. Phase 1, 3, and 4 are complete.
- Next natural target: `xetex-errors.c` (7 fns, 175 LOC). Two functions are
  static (pre_error_message, post_error_message); they can be translated as
  private Rust helpers. `error()` calls `show_context()` and `give_err_help()`
  which are still in C — call via FFI.
- `xetex-errors.c` needs new globals: `log_opened`, `job_name`, `interaction`,
  `error_count`, `use_err_help`, `history`, `halt_on_error_p`, and extern FFI
  for `open_log_file`, `show_context`, `give_err_help`, `close_files_and_terminate`,
  `tt_cleanup`, `ttstub_output_flush`.

---

## Session 9 — 2026-04-18 — Phase 3 complete: xetex-output.c fully translated

**Goal:** translate the remaining 11 functions from `xetex-output.c` and mark
the module DONE.

**Starting state:**
- Parity: 29/29
- Streak: 8

**Work done:**
- Fixed two bugs carried over from previous context: `print_native_word` used
  non-existent `raw_mem_ptr()` (replaced with new `mem_as_u16_ptr()` accessor in
  globals.rs), and `print_sa_num` used `mem_b32s0(q) >> 16` where `mem_b16s1(q)`
  was needed, plus had spurious `% 64` on the n1 term. (3786ec64, 1d48b016)
- Added globals: `mem_as_u16_ptr`, `set_current_diagnostic`, `in_open`,
  `full_source_filename_stack`, `line_stack`, `file_line_error_style_p`,
  `hash_s1`, `prim_s1`, `eqtb_top`. (3786ec64)
- Translated: `capture_to_diagnostic`, `diagnostic_print_file_line` (private
  Rust helper), `diagnostic_begin_capture_warning_here`,
  `error_here_with_diagnostic`, `print_file_line`, `print_size`,
  `print_write_whatsit`, `print_file_name`, `print_cs`, `sprint_cs`,
  `print_native_word`, `print_sa_num`. (1d48b016)
- Deleted all C bodies from `xetex-output.c`; only variable definition
  `current_diagnostic` remains as C-side storage. (47572258)
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 9

**Surprises / gotchas:**
- `print_sa_num` n1 term has no `% 64` in C (only n0 does) — easy to miss.
- `NATIVE_NODE_SIZE = 6` is in `xetex-constants.h:130`; needed `mem_as_u16_ptr`
  helper returning `raw::mem.offset(p) as *const u16` to avoid exposing raw mem.
- Session started mid-work (context compaction); first task was to fix bugs in
  code written at end of previous context window.

**For the next session:**
- `xetex-output.c` is DONE. Phase 3 is complete.
- Next target: `xetex-stringpool.c` (206 LOC, 8 functions) or `xetex-errors.c`
  (175 LOC, 7 functions) — both are small and self-contained.
- `m_log` in `xetex-scaledmath.c` is now unblocked (Phase 3 diagnostic machinery
  is all in Rust); could be translated as a warm-up next session.

---

## Session 8 — 2026-04-17 — Phase 3 bulk: print_nl through print_current_string

**Goal:** translate all remaining Phase 3 functions that don't require the
`diagnostic_print_file_line` static helper or `print_cs`/`sprint_cs` complex globals.

**Starting state:**
- Parity: 29/29
- Streak: 7

**Work done:**
- Confirmed parity baseline: 29/29.
- Commit `4f7ad6bd`: translated `print_nl` (output.c:302) and `print_esc` (output.c:310).
- Commit `a4e5e260`: translated `print_two` (554), `print_roman_int` (563), `print_scaled` (612).
- Commit `d7e39b4d`: prep — un-static'd `warn_char` and `current_diagnostic`.
- Commit `392e3220`: translated `warn_char` (75) and `print_ln` (84).
  Added `selector`/`term_offset`/`file_offset` setters, `rust_stdout`/`log_file`/`write_file`/
  `current_diagnostic` globals.
- Commit `7b8944cf`: translated `print_raw_char` (76).
  Added `max_print_line`, `tally`, `error_line`, `trick_buf`, `pool_ptr`, `pool_size`,
  `str_pool` globals.
- Commit `62cd93b1`: translated `print_char` (76). Added `doing_special` global.
- Commit `13671cf5`: translated `print` (146). Added `str_ptr`, `str_start`,
  `str_pool_entry`, `set_eqtb_b32s1` globals.
- Commit `16cdc895`: translated `print_current_string` (328). Trivial after above.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 8
- 49 functions migrated total.
- xetex-output.c: 329 LOC (~7 functions) remaining — the diagnostic cluster
  (capture_to_diagnostic, diagnostic_begin_capture_warning_here,
  error_here_with_diagnostic, diagnostic_print_file_line) and print_cs/sprint_cs/
  print_file_name/print_file_line.

**Surprises / gotchas:**
- `print_char` in C had a dead `small_number l` variable that is actually used in
  the s < 160 branch for hex encoding — caught at translation time.
- `print_raw_char` takes `UTF16_code s` (u16) not `int32_t` — the Rust signature
  must match exactly since it's called from C callers that pass short values.
- `print` has `INTPAR(new_line_char) = -1` which requires a new `set_eqtb_b32s1`
  setter. Be careful not to confuse eqtb read vs. write paths.

**For the next session:**
- **Next targets (Phase 3 remainder):**
  1. `capture_to_diagnostic` — needs `ttstub_diag_finish` extern; straightforward.
  2. `diagnostic_print_file_line` (static) — needs `in_open`, `full_source_filename_stack`,
     `line_stack` globals and `ttstub_diag_printf` (variadic!). Option: un-static it
     and call via FFI from the two functions that use it, or translate using a
     Rust format-then-append approach.
  3. `diagnostic_begin_capture_warning_here` + `error_here_with_diagnostic` —
     depend on `diagnostic_print_file_line`.
  4. `print_file_line` — needs `in_open`, `full_source_filename_stack`, `line_stack`;
     similar to `diagnostic_print_file_line`.
  5. `print_cs` / `sprint_cs` — need `HASH_BASE`, `SINGLE_BASE`, `NULL_CS`,
     `hash[]`, `prim[]`, `eqtb_top`, `str_ptr`, `CAT_CODE` macro,
     `ACTIVE_BASE`, `PRIM_EQTB_BASE`, `FROZEN_NULL_FONT`. Heavy on constants.
- Alternative quickest path: translate `print_file_name` and see if it's simpler.

---

## Session 7 — 2026-04-17 — Phase 3: print_cstr, print_nl_cstr, print_esc_cstr

**Goal:** translate the three simple Phase 3 wrapper functions that have no
static-function dependencies.

**Starting state:**
- Parity: 29/29
- Streak: 6

**Work done:**
- Confirmed parity baseline: 29/29.
- Added `selector`, `term_offset`, `file_offset` accessors to `src/globals.rs`.
- Added constants `SELECTOR_LOG_ONLY`, `INT_BASE`, `INT_PAR_ESCAPE_CHAR`,
  `BIGGEST_USV` to `src/output.rs`.
- Commit `555d6b46`: translated `print_cstr` (output.c:301), `print_nl_cstr`
  (output.c:319), `print_esc_cstr` (output.c:338). Deleted ~35 C lines.
  Removed old `extern "C" { fn print_cstr }` declaration; added `print_ln`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 7

**Surprises / gotchas:**
- `INTPAR(escape_char)` expands to `eqtb[INT_BASE + INT_PAR__escape_char].b32.s1`
  where `INT_BASE = 7_826_729` and `INT_PAR__escape_char = 45`. Used the
  existing `eqtb_b32s1` accessor from globals.rs directly.
- `odd(selector)` in the C header is `(x) & 1`; translated as `sel & 1 != 0`.

**For the next session:**
- Next natural targets in Phase 3: `print_nl` (calls `print_ln` + `print`,
  both in C), `print_esc` (calls `print_char` + `print`).
  These are straightforward since their callees are all extern C.
- After those: `print_two` (line 649), `print_roman_int` (line 675),
  `print_scaled` (line 724) — all pure output functions.
- The `warn_char` / `print_ln` / `print_raw_char` cluster still requires
  un-static-ing `warn_char` first. Defer until the simpler wrappers are done.

---

## Session 6 — 2026-04-16 — Phase 3 start: print_int, print_hex, print_ucs_code

**Goal:** begin Phase 3 (printing functions from xetex-output.c) with the
cleanest targets that have no static-function dependencies.

**Starting state:**
- Parity: 29/29
- Streak: 5

**Work done:**
- Read 04-migration-order.md and 07-anti-patterns.md. Oriented per workflow.
- Confirmed parity baseline: 29/29.
- Identified that `warn_char` (static, called by `print_ln`/`print_raw_char`)
  blocks translating the core printing functions. Deferred those.
- Identified `print_int`, `print_hex`, `print_ucs_code` as clean targets:
  they only use the `dig[]` array and call C `print_char`/`print_cstr`.
- Created `src/output.rs`. Added `dig[]` accessors to `globals.rs`.
- Commit `6e9a8bea`: translated `print_the_digs` (private Rust helper),
  `print_int`, `print_hex`, `print_ucs_code`. Deleted 84 C lines.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 6

**Surprises / gotchas:**
- `warn_char` is a static C function that reads `current_diagnostic` (also
  a C file-static). Functions that call `warn_char` (`print_ln`,
  `print_raw_char`) cannot be translated until `warn_char` is either
  un-static'd (prep commit) or translated alongside `capture_to_diagnostic`
  (which owns `current_diagnostic`). The whole cluster must move together.
- `print_the_digs` was called by `print_int`, `print_hex`, AND `print_ucs_code`
  so all three had to be translated in the same commit (otherwise the C
  static becomes unreachable from its callers).

**For the next session:**
- Next targets in Phase 3: `print_ln` + `print_raw_char` require un-static-ing
  `warn_char` first. Plan: prep commit to un-static `warn_char` + `current_diagnostic`
  (make `current_diagnostic` an extern, add prototype for `warn_char`), then
  translate `warn_char` to Rust, then `print_ln` and `print_raw_char`.
- Alternatively: translate the simpler wrappers `print_cstr`, `print_nl_cstr`,
  `print_esc_cstr` (these call only C extern functions, no static deps) — quick wins.

---

## Session 5 — 2026-04-16 — Phase 2: copy_native_glyph_info

**Goal:** translate `copy_native_glyph_info` from xetex-xetex0.c to `src/mem.rs`.

**Starting state:**
- Parity: 29/29
- Streak: 4

**Work done:**
- Confirmed parity baseline: 29/29.
- Added `mem_ptr`/`set_mem_ptr` accessors to `src/globals.rs` (for `mem[p].ptr`, the `void*` field).
- Added `xmalloc` extern declaration to `src/mem.rs`.
- Commit `37480639`: translated `copy_native_glyph_info` → uses `xmalloc` + `ptr::copy_nonoverlapping`; deleted 14-line C body.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 5

**Surprises / gotchas:**
- `NATIVE_GLYPH_INFO_SIZE` is in `xetex-constants.h`, not the main header — had to grep for it.
- `xmalloc_array(char, n)` expands to `xmalloc((n+1) * sizeof(char))` = `xmalloc(n+1)` — the +1 is for null-terminator safety.

**For the next session:**
- Phase 2 table in MODULE_STATUS.md is fully migrated (all 17 functions done). Next is Phase 3 (printing functions from xetex-output.c), or resume with more xetex-xetex0.c functions not yet in the tracker.

---

## Session 4 — 2026-04-16 — Phase 2: 12 new_* node constructors

**Goal:** translate the 12 `new_*` node constructor functions from xetex-xetex0.c to `src/mem.rs`.

**Starting state:**
- Parity: 29/29
- Streak: 3

**Work done:**
- Confirmed parity baseline: 29/29.
- Added `eqtb`, `temp_ptr` globals and mem b16/gr/word accessors to `src/globals.rs`.
- Commit `8fef23ac`: translated new_null_box, new_rule, new_ligature, new_lig_item, new_disc, new_math, new_spec, new_param_glue, new_glue, new_skip_param, new_kern, new_penalty into `src/mem.rs`.
  Deleted corresponding C bodies from xetex-xetex0.c (~142 lines removed).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 4

**Surprises / gotchas:**
- First deletion Edit omitted the function header for new_null_box, leaving a dangling `int32_t new_null_box(void)\n{\n    int32_t p;\n` fragment in the C file. Caused cascading parse errors. Fixed with a second Edit removing the stray header. Always include the full function signature in deletion patterns.

**For the next session:**
- Next target: `copy_native_glyph_info` (xetex-xetex0.c ~line 359) — uses xmalloc_array + memcpy + NATIVE_NODE macros; more complex than the new_* constructors.
- After that: remaining Phase 2 functions deeper in xetex-xetex0.c.

---

## Session 3 — 2026-04-16 — Phase 2 kick-off: badness + core allocators

**Goal:** start Phase 2 (memory/node allocation from xetex-xetex0.c)
by translating the core allocator functions.

**Starting state:**
- Parity: 29/29
- Streak: 2

**Work done:**
- Confirmed parity baseline: 29/29.
- Commit `e19ace3f`: translated badness, get_avail, flush_list.
  - `badness` (xetex-xetex0.c:25) → `src/helpers.rs` (new file).
  - `get_avail` (xetex-xetex0.c:170) → `src/mem.rs`.
  - `flush_list` (xetex-xetex0.c:192) → `src/mem.rs`.
  - New globals: `mem` (raw pointer), `lo_mem_max`, `hi_mem_min`,
    `avail`, `mem_end`, `rover` + `mem_b32s0`/`mem_b32s1` typed
    accessor pairs. TEX_NULL, MEM_TOP, MAX_HALFWORD as Rust consts.
  - get_avail's OOM path calls C `runaway()`/`overflow()` via
    extern "C" (not yet translated).
- Commit `fc914449`: translated free_node and get_node.
  - `free_node` (xetex-xetex0.c:284) — straightforward list insertion.
  - `get_node` (xetex-xetex0.c:150) — the main lo-mem allocator; two
    C `goto` labels handled via Rust labeled-loop break/continue:
    `goto found` → `break 'restart r`, `goto restart` → `continue 'restart`.
  - New globals: `InputState` (#[repr(C)] struct for `input_state_t`),
    `cur_input_synctex_tag()`, `line()`. Added `set_lo_mem_max`.
  - New module `src/helpers.rs` registered in `lib.rs`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 3
- 18 functions migrated total (13 from scaledmath + 5 from xetex0).
- xetex-xetex0.c: 18,543 LOC, ~268 functions remaining.

**Surprises / gotchas:**
- Rust unit test runner shows "running 0 tests" in the doc-tests pass
  (a second binary) — that's normal. The main test binary ran 21 tests.
- `get_node` Case 133 (exact-fit block): the C code checks
  `if (r == p)` first and then `if (mem[p+1].b32.s1 != p)` as a
  second condition. I collapsed these into `if r == p && mem_b32s1(p + 1) != p`.
  Semantically identical because the inner if can only be true when
  the outer is true.
- `InputState` must exactly mirror `input_state_t` (state+index are
  u16, then five i32 fields). Any field reorder or padding difference
  would silently corrupt `cur_input.synctex_tag` reads. The `#[repr(C)]`
  attribute plus the field ordering matches the C definition at
  xetex-xetexd.h:344–352.

**For the next session (Session 4):**
- **Priority 1:** continue Phase 2 — the `new_*` node constructors.
  These all call `get_node` (now in Rust!) so the call chain is clean.
  Start with the simplest: `new_null_box` (line 223 in current file),
  `new_rule` (~240), `new_ligature` (~253), `new_lig_item` (~265),
  `new_disc` (~273), `new_math` (~299), `new_spec` (~309),
  `new_param_glue` (~321), `new_glue` (~336), `new_skip_param` (~349),
  `new_kern` (~360), `new_penalty` (~370).
  These need additional mem accessors (b16 fields, gr field).
- **Note:** `new_null_box` uses `mem[p].b16.s0`, `BOX_glue_set`,
  `NODE_type` — check `xetex-xetexd.h` for the macro definitions before
  translating. Will need `mem_b16s0`/`set_mem_b16s0` etc.
- **Priority 2:** once the `new_*` constructors are done, `copy_native_glyph_info`
  is next, followed by Phase 3 (output/printing).

---

## Session 2 — 2026-04-16 — Complete xetex-scaledmath.c (minus m_log)

**Goal:** translate the remaining 7 translatable functions from
`xetex-scaledmath.c` (make_frac, take_frac, ab_vs_cd, new_randoms,
init_randoms, unif_rand, norm_rand).

**Starting state:**
- Parity: 29/29
- Streak: 1

**Work done:**
- Confirmed parity baseline: 29/29.
- Prep commit `1006768f`: removed `static` from make_frac, take_frac,
  m_log, ab_vs_cd, new_randoms in xetex-scaledmath.c. Added prototypes
  to xetex-xetexd.h. Build verified.
- Added `randoms[55]` and `j_random` global accessors to `src/globals.rs`
  (random_entry / set_random_entry / j_random / set_j_random).
- Translated 7 functions to Rust in `src/scaled_math.rs`, removed their
  C bodies — commit `3f096961`:
    - `make_frac` (xetex-scaledmath.c:165)
    - `take_frac` (xetex-scaledmath.c:220)
    - `ab_vs_cd` (xetex-scaledmath.c:332)
    - `new_randoms` (xetex-scaledmath.c:418)
    - `init_randoms` (xetex-scaledmath.c:441)
    - `unif_rand` (xetex-scaledmath.c:468)
    - `norm_rand` (xetex-scaledmath.c:487) — calls C `m_log` via
      `extern "C"` declaration in scaled_math.rs.
- xetex-scaledmath.c now contains only m_log (66 LOC).
- All 15 crate-level unit tests pass; parity 29/29 after switch.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 2
- `MODULE_STATUS.md` shows xetex-scaledmath.c IN PROGRESS — 93%
  (13/14 functions migrated, 1 blocked).

**Surprises / gotchas:**
- All 7 functions ended up in one commit because they were written
  together in scaled_math.rs before the C bodies were deleted. The
  workflow says "one function per commit when practical" — this was
  a pragmatic deviation, acceptable since they form a tight cluster.
- `norm_rand` calls `m_log` (blocked on Phase 3). To keep `norm_rand`
  in Rust now, m_log was un-static'd in C (so the FFI call links).
  This is clean: norm_rand will need no changes when m_log is eventually
  translated; we just remove the `extern "C" { fn m_log }` declaration.
- The `take_frac` overflow check uses `n - 0x7FFFFFFF` (C `be_careful`);
  since n can be 0x7FFFFFFF and p can be positive, `be_careful + p > 0`
  can overflow. The C code relies on wrapping behavior here — we don't
  need `wrapping_*` because n is set to `0x7FFFFFFF - p` in the overflow
  branch, making the subsequent addition exact.

**For the next session (Session 3):**
- **Priority 0:** parity baseline 29/29 (already solid).
- **Priority 1:** start Phase 2 — memory / node allocation helpers from
  `xetex-xetex0.c`. Candidates (all in xetex-xetex0.c):
    - `badness` (line 25 in xetex-xetex0.c per session-1 notes) —
      pure math, no globals except possibly `arith_error`.
    - `get_avail` (line 170), `flush_list` (line 192) — memory helpers.
  Read `02-state.md` and `03-memory-and-nodes.md` before starting Phase 2.
- **Priority 2:** once xetex-scaledmath.c is the only remaining C in
  the file (it is!), we could mark it DONE if m_log is translated.
  But m_log is BLOCKED — leave as IN PROGRESS.
- **Do NOT** touch m_log until the diagnostic/error machinery
  (error_here_with_diagnostic, print_scaled, etc.) is in Rust (Phase 3).

---

## Session 1 — 2026-04-16 — Phase 0 scaffolding + first Phase 1 translations

**Goal:** establish the parity baseline, set up the Rust scaffolding
(`src/globals.rs`, `src/mem_types.rs`, `src/mem.rs`), and translate
the pure arithmetic helpers from `xetex-scaledmath.c`.

**Starting state:**
- Parity: unmeasured at start.
- Streak: 0

**Plan corrections applied (pre-code):**
- `MODULE_STATUS.md` summary said 14 files; the table listed 16.
  Fixed the summary and filled in `~??` LOC for errors.c (175),
  ext.c (2170), linebreak.c (2843), io.c (555), engine-interface.c
  (112).
- `02-state.md` had an unsound example (`mem_slice_mut` returning
  `&'static mut [MemoryWord]` — two calls would alias). Replaced
  with a by-value `mem_word`/`set_mem_word` pair and documented
  the "no live references across calls" invariant.
- `PROGRESS.md` claimed 49,814 C LOC; actual in-scope total is
  39,419. Corrected.
- `01-architecture.md` "Current layout" section now lists accurate
  LOC for every in-scope file, and calls out `xetex-macos.c` as
  the conditional mac-only file.
- `06-workflow.md` Step 4 now covers `static` C functions: either
  translate caller+callee together, or un-static the C definition
  as a prep commit. This came up because most of Phase 1's
  leftovers (`make_frac`, `take_frac`, `m_log`, `ab_vs_cd`,
  `new_randoms`) are `static`.
- Created `plan/scripts/parity.sh` per `05-testing.md`. Logs now
  land in `plan/parity-history.log`.

**Work done:**
- Recorded parity baseline: **29 passed / 29 total** (session 0
  baseline).
- Created `src/mem_types.rs` with `MemoryWord`, `B32x2`, `B16x4`
  (`#[repr(C)]`, compile-time `size_of` asserts).
- Created `src/mem.rs` with `MemIdx`, `Scaled`, `FontId`,
  `StrNumber`, `Token` newtypes. No accessors yet — per the
  Phase-0 plan, those come alongside their first use in Phase 2.
- Created `src/globals.rs` with safe accessors for `arith_error`
  and `tex_remainder`. Declared the extern statics inside a
  `mod raw` to keep them out of the wrapper namespace, with a
  module-level SAFETY note forbidding any accessor that returns
  a live reference.
- Translated six pure functions from `xetex-scaledmath.c` into
  `src/scaled_math.rs`:
  - `tex_round` (xetex-scaledmath.c:11)
  - `half` (xetex-scaledmath.c:51)
  - `mult_and_add` (xetex-scaledmath.c:60)
  - `x_over_n` (xetex-scaledmath.c:79)
  - `xn_over_d` (xetex-scaledmath.c:105)
  - `round_xn_over_d` (xetex-scaledmath.c:137)
  Each carries a `// translated from …` citation and the six C
  definitions in `xetex-scaledmath.c` have been removed (header
  prototypes in `xetex-xetexd.h:1130–1134` and 763 remain, so C
  callers link to the Rust `#[no_mangle]` impls).
- Wrote 15 unit tests covering sign conventions, overflow flags,
  rounding, and zero-divisor behaviour.
- Parity held at 29/29 after the switch.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 1
- `MODULE_STATUS.md` phase-1 tracker reflects six migrated
  functions; `xetex-scaledmath.c` is "IN PROGRESS — 43%"
  (6/14 functions).

**Surprises / gotchas:**
- Rust debug-mode overflow checks panic where C silently wraps.
  Three concrete spots needed `wrapping_*` variants:
    (a) `mult_and_add`'s `max_answer ± y` boundary test,
    (b) `xn_over_d` / `round_xn_over_d`'s 32-bit intermediates
        which rely on C `32768L` (long=64) for wide compute,
    (c) the `2*v` comparison in `round_xn_over_d`.
  Comments in the source explain the reason per site.
- Initial unit tests used `i32::MAX` as `max_answer` for
  `mult_and_add`; that's outside the algorithm's implicit
  `max_answer + y fits in i32` precondition. Real TeX callers
  use `MAX_HALFWORD = 0x3FFFFFFF`. Tests updated to reflect that.
- The extern static and the safe wrapper of the same name in one
  module collide; putting the `extern "C"` block in a private
  `mod raw` resolves it cleanly without renaming the public API.
- A literal note for future sessions: the C in `x_over_n` contains
  a dead `tex_remainder = -tex_remainder;` in the `n < 0` branch
  (the value is always overwritten below). We preserve it
  faithfully to keep the translation-traceability property.

**For the next session (Session 2):**
- **Priority 0:** run `./plan/scripts/parity.sh session-2-start`
  and confirm 29/29.
- **Priority 1:** translate the remaining `xetex-scaledmath.c`
  functions **except** `m_log`. Targets:
    - `make_frac`, `take_frac`, `ab_vs_cd` (pure, `static` in C —
      un-static them first, then translate).
    - `new_randoms`, `init_randoms`, `unif_rand`, `norm_rand`
      (need the `randoms[]` and `j_random` globals added to
      `src/globals.rs` — note `randoms` is an array, so the safe
      wrapper must index by element).
- **Priority 2 (if time allows):** start on `badness` and `half`-
  adjacent helpers from `xetex-xetex0.c`. Line numbers:
  `badness` at xetex-xetex0.c:25.
- **Do NOT yet** attempt `m_log` — it depends on the diagnostic
  machinery (`error_here_with_diagnostic`, `print_scaled`,
  `print_cstr`, `capture_to_diagnostic`, `error`, `help_*`) which
  is Phase 3.

---

## Session 0 — 2026-04-15 — Plan scaffolding

**Goal:** set up `plan/` directory with strategy docs and tracking
scaffolding so that subsequent sessions have a clear, self-contained
workflow.

**Starting state:**
- Parity: not yet measured.
- Streak: 0

**Work done:**
- Surveyed `crates/engine_xetex/xetex/` (14 C files in scope,
  ~50k LOC). Noted `xetex-xetex0.c` is ~18,700 LOC / 273 functions
  and `xetex-ini.c` is ~3,900 LOC and hosts the global state
  definitions.
- Surveyed `crates/engine_xetex/src/lib.rs` (267 LOC, thin FFI
  wrapper around `tt_engine_xetex_main`).
- Surveyed `tests/tex-outputs/` (~99 files, ~25–30 test cases
  comparing .xdv/.log/.pdf against goldens).
- Wrote the plan documents:
  - `README.md` — navigation + hard rules
  - `00-overview.md` — strategy and principles
  - `01-architecture.md` — target crate layout and FFI interleaving
  - `02-state.md` — state-modeling migration strategy
  - `03-memory-and-nodes.md` — arena & node typing model
  - `04-migration-order.md` — dependency-ordered translation queue
  - `05-testing.md` — parity harness and golden-test strategy
  - `06-workflow.md` — per-session mechanical checklist
  - `07-anti-patterns.md` — mistakes to never repeat (critical)
  - `MODULE_STATUS.md` — per-file status tracker
  - `PROGRESS.md` — test-parity dashboard
  - `SESSION_LOG.md` — this file

**Ending state:**
- Parity: still not measured.
- Streak: 0
- No Rust code written yet. No C code deleted yet.

**Surprises / gotchas:**
- The C source retains a lot of `memory_word`-based bit packing (see
  `xetex-xetexd.h:85–196`). Translating node accessors is the main
  mechanical challenge in early phases — it all hinges on
  `MemoryWord` being exactly as described in `03-memory-and-nodes.md`.
- `xetex-xetex0.c` is 18,700 lines of one monolithic translation
  unit. It will not map to a single Rust module; the phase plan in
  `04-migration-order.md` splits it by function role.
- Tests do not live in `engine_xetex` itself; they live in the
  top-level `tectonic` crate at `tests/tex-outputs.rs`. This means
  the parity command is run at repo root against the top-level
  crate, which pulls `engine_xetex` in transitively.

**For the next session (Session 1):**
- **Priority 0:** run the parity baseline. Record it as "Session 0"
  in `PROGRESS.md` so subsequent sessions have a floor.
- **Priority 1:** count C functions per file with the grep snippet
  in `MODULE_STATUS.md` and fill in the `Fns (C)` column.
- **Priority 2:** set up `src/globals.rs` — a centralized Rust module
  that declares `extern` bindings to every C global listed in
  `xetex-xetexd.h`. This is the foundation for every Phase 1
  translation. See `02-state.md` §"Phase A" for the exact shape.
- **Priority 3:** if time remains, translate the first pure-helper
  function (`badness`? `half`?) from `xetex-scaledmath.c` as a
  proof-of-concept. See `04-migration-order.md` §"Phase 1".
- Do NOT yet touch `xetex-xetex0.c` — start with the small pure
  files so the build/test/commit loop is proven out on low-risk code.
