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

## Session 85 — 2026-05-07 — xetex-ext.c font getter helpers

**Goal:** Continue `xetex-ext.c` by translating the cohesive
OpenType/Graphite font-query and metrics helper cluster.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 84

**Work done:**
- Translated nine `xetex-ext.c` helpers to `src/ext.rs`:
  `release_font_engine`, `ot_get_font_metrics`, `ot_font_get`,
  `ot_font_get_1`, `ot_font_get_2`, `ot_font_get_3`,
  `gr_print_font_name`, `gr_font_get_named`, and `gr_font_get_named_1`.
- Added the matching layout-engine FFI declarations and mirrored the
  XeTeX command-code constants used by the scanner (`XETEX_*_CODE -
  XETEX_INT`).
- Preserved Graphite label ownership by printing through Rust
  `print_cstr` and then calling `gr_label_destroy`, matching the C
  helper behavior.
- Reduced `xetex-ext.c` from 1,773 LOC / ~42 C bodies to 1,582 LOC /
  ~33 C bodies.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 85

**Surprises / gotchas:**
- The Mac `AAT_FONT_FLAG` branch in `release_font_engine` must remain
  cfg-gated on Linux to avoid adding a new dead-code warning while still
  preserving the C branch shape for macOS.
- The Graphite label APIs return raw labels whose destruction remains tied
  to `gr_label_destroy`; do not switch these to `free`/`CString::from_raw`
  from the `engine_xetex` side.

**For the next session:**
- Continue `xetex-ext.c`. Good next candidates are the small native glyph
  and character metric wrappers (`get_native_char_height_depth`,
  `get_native_char_sidebearings`, `getnativechar*`, `get_glyph_bounds`,
  `map_char_to_glyph`, `map_glyph_to_index`, `get_font_char_range`,
  `print_glyph_name`) before the larger glyph-array and native-node
  measurement routines.

---

## Session 84 — 2026-05-07 — xetex-ext.c parser helpers

**Goal:** Continue `xetex-ext.c` by translating the small encoding and
font-feature parsing helpers that still feed the C font-loading path.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 83

**Work done:**
- Translated `get_encoding_mode_and_info` to `src/ext.rs`.
- Translated three former C file-static helpers to Rust while leaving C
  prototypes for the remaining C callers:
  `read_tag_with_param`, `readFeatureNumber`, and `splitFontName`.
- Added focused unit tests for `readFeatureNumber` and `splitFontName`.
- Reduced `xetex-ext.c` from 1,915 LOC / ~47 C bodies to 1,773 LOC / ~42 C
  bodies.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 84

**Surprises / gotchas:**
- Direct `ucnv_open`/`ucnv_close` declarations built the library but failed
  when linking the full `tectonic` test binary. The Rust translation now uses
  the existing `xetex_ucnv_open`/`xetex_ucnv_close` bridge functions from
  `xetex-io.c`, which handle the ICU symbol setup already used elsewhere.
- `readFeatureNumber` permits whitespace before `=`, but not after it; the
  unit test was adjusted to match the C parser exactly.

**For the next session:**
- Continue `xetex-ext.c`. Good candidates are smaller OpenType/Graphite
  getters and printing helpers (`ot_font_get*`, `gr_font_get*`,
  `gr_print_font_name`) before attempting the larger `loadOTfont` or
  native-node measurement paths.

---

## Session 83 — 2026-05-07 — xetex-ext.c mapping feature helpers

**Goal:** Continue `xetex-ext.c` by moving the top-of-file printing and
mapping feature helper cluster into Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 82

**Work done:**
- Translated `print_utf8_str` and `print_chars` to `src/ext.rs`.
- Translated the TECkit mapping helper cluster:
  `check_for_tfm_font_mapping`, `load_tfm_font_mapping`,
  `apply_tfm_font_mapping`, and private `load_mapping_file`.
- Translated `readCommonFeatures`, including the mapping, extend, slant,
  embolden, letterspace, and color parsing branches.
- Moved the former C file-static `saved_mapping_name` into Rust as
  `SAVED_MAPPING_NAME`.
- Added `loaded_font_mapping` and `loaded_font_flags` setters in
  `src/globals.rs` so `readCommonFeatures` can update the same engine state
  as the C implementation.
- Reduced `xetex-ext.c` from 2,088 LOC / 55 rough C functions to 1,915 LOC /
  47 rough C functions.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 83

**Surprises / gotchas:**
- `load_mapping_file` was file-static in C, but it had to move with
  `readCommonFeatures`; otherwise the translated Rust caller could not link to
  a C-local helper.
- Declaring `_tt_abort` as variadic in `src/ext.rs` introduced a new
  clashing-extern warning. The Rust code now uses the existing one-argument
  declaration style and a fixed fatal message on mapping-file read failure.
- `UTF16_NATIVE` is endian-dependent, so the Rust translation uses
  `#[cfg(target_endian = ...)]` instead of a runtime branch.

**For the next session:**
- Continue `xetex-ext.c`. Reasonable candidates are `readFeatureNumber` and
  `splitFontName`, then the smaller OpenType/Graphite getter and printing
  helpers before taking on `loadOTfont` or native-node measurement.

---

## Session 82 — 2026-05-07 — xetex-ext.c helper slice

**Goal:** Start `xetex-ext.c` by translating a small, low-state helper group
used by feature parsing and picture/font fixed-point conversion.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 81

**Work done:**
- Created `src/ext.rs` and added it to `src/lib.rs`.
- Translated four public helpers from `xetex-ext.c`:
  - `read_double` (xetex-ext.c:279)
  - `read_rgb_a` (xetex-ext.c:344)
  - `D2Fix` (xetex-ext.c:1882)
  - `Fix2D` (xetex-ext.c:1888)
- Added focused unit tests for `read_double` and `read_rgb_a` parsing behavior.
- Removed the corresponding C bodies while keeping the existing header
  prototypes for C callers.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 82

**Surprises / gotchas:**
- `read_double` accumulates decimal digits exactly like the C loop, so unit
  tests must use tolerance rather than exact `f64` equality.
- The first focused `cargo test -p tectonic_engine_xetex ext::tests` needed
  escalation because Cargo had to write to the workspace-level target directory.

**For the next session:**
- Continue `xetex-ext.c`. Good next candidates are still near the top of the
  file: `print_utf8_str`, `print_chars`, and the TFM mapping wrappers, before
  taking on the larger native-font loading/layout functions.

---

## Session 81 — 2026-05-07 — xetex-synctex.c

**Goal:** Translate `xetex-synctex.c` (1,283 LOC, ~30 functions) to `src/synctex.rs`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 80

**Work done:**
- Created `src/synctex.rs` with Rust translations of all ~30 functions from
  `xetex-synctex.c`. Includes the public API (`synctex_init_command`,
  `synctex_terminate`, `synctex_abort`, `synctex_start_input`, `synctex_sheet`,
  `synctex_teehs`, `synctex_pdfxform`, `synctex_mrofxfdp`, `synctex_vlist`,
  `synctex_tsilv`, `synctex_void_vlist`, `synctex_hlist`, `synctex_tsilh`,
  `synctex_void_hlist`, `synctex_math`, `synctex_horizontal_rule_or_glue`,
  `synctex_kern`, `synctex_pdfrefxform`, `synctex_current`) and all private
  helpers (`synctex_dot_open`, `synctex_prepare_content`, `get_current_name`,
  `synctexabort`, `synctex_record_*`). Added `mod synctex;` to `lib.rs`.
  Reduced `xetex-synctex.c` to a 10-line stub of includes.
- `MODULE_STATUS.md`: `xetex-synctex.c` NOT STARTED → DONE. Summary:
  NOT STARTED 5→4, DONE 10→11.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 81

**Surprises / gotchas:**
- `ttstub_fprintf` is variadic — replaced with `format!` + `ttstub_output_write`.
  The `synctex_write` helper returns written bytes as `i32` (matching C `int`).
- The file-static `synctex_ctxt` C struct uses bitfields for flags. Replaced
  with a plain Rust struct with `bool` fields (no layout constraint since
  it's private state). Stored in `static mut SYNCTEX_CTXT: SynctexCtxt`.
- The recorder function pointer (`synctex_record_node_kern`) is stored as
  `Option<unsafe fn(i32)>` and called by extracting `(recorder, node)` before
  calling, to avoid aliased `&mut SYNCTEX_CTXT` references during the call.
- `~counter > 0` (C bitwise NOT) translates to `counter != u32::MAX` in Rust.
- `synctex_abort` is declared in the header but not implemented in the C file;
  implemented here as a thin wrapper around `synctexabort()`.
- `synctex_record_preamble` and `synctex_record_anchor` use `total_length = len`
  (assignment, not `+=`) matching a quirk in the original C source.

**For the next session:**
- Remaining NOT STARTED files: `xetex-ini.c` (3,881 LOC), `xetex-math.c`
  (3,833 LOC), `xetex-shipout.c` (2,451 LOC), `xetex-ext.c` (2,170 LOC).
  IN PROGRESS: `xetex-engine-interface.c` (85 LOC, 2 fns stay in C).
- Recommend `xetex-ext.c` next — it contains `Fix2D`/`D2Fix` used by pic.rs
  and other modules.

---

## Session 80 — 2026-05-07 — xetex-linebreak.c cleanup + xetex-pic.c

**Goal:** Remove the disabled `#if 0` reference blocks from
`xetex-linebreak.c`, then translate `xetex-pic.c` to Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 79

**Work done:**
- Deleted both disabled `#if 0` blocks (the `line_break` and `try_break`
  reference bodies) from `xetex-linebreak.c`, along with the macro defines
  (`AWFUL_BAD`, `VERY_LOOSE_FIT`, `LOOSE_FIT`, `DECENT_FIT`, `TIGHT_FIT`,
  `LAST_ACTIVE`) and file-local statics (`second_pass`, `fewest_demerits`,
  `actual_looseness`, `line_diff`, `init_cur_lang`, `last_line_fill`) that
  were only used inside those blocks. The non-static globals that Rust
  accesses via `extern "C"` were retained. Commit 3c2b4116.
- Created `src/pic.rs` with Rust translations of all 11 functions from
  `xetex-pic.c`: `count_pdf_file_pages`, `load_picture`, and their 9
  private helpers (`pdf_get_rect`, `get_image_size_in_inches`,
  `find_pic_file`, `transform_point`, `make_identity`, `make_scale`,
  `make_translation`, `make_rotation`, `transform_concat`). All dpx/bridge
  calls remain via `extern "C"`. `Fix2D`/`D2Fix` still call into
  `xetex-ext.c` via FFI. `xetex-pic.c` is now a 48-LOC stub of includes.
  Commit cb2b9ec9.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 80

**Surprises / gotchas:**
- `pdf_tmatrix` in dpx has fields `a, b, c, d, e, f` (six doubles), not
  `a, b, c, d, x, y`. The Rust struct must match this layout exactly.
- The xetex-pic.c `transform_t` local struct uses `x, y` for translation
  while dpx's `pdf_tmatrix` uses `e, f`. These are different types that
  happen to be used together in `pdf_get_rect`.
- `load_picture`'s `make_translation` call uses `-(int32_t) xmin` casts:
  the C code first truncates the float to int before negating. The Rust
  translation preserves this with `-(xmin as i32 as f64)`.

**For the next session:**
- Continue with the next NOT STARTED file. Candidates by size: `xetex-synctex.c`
  (1,283 LOC), `xetex-ext.c` (2,170 LOC), `xetex-shipout.c` (2,451 LOC),
  `xetex-math.c` (3,833 LOC), `xetex-ini.c` (3,881 LOC).
- Per the migration order, Phase 7 (math) is next, but `xetex-synctex.c`
  is much smaller. A session on synctex would be tractable.

---

## Session 79 — 2026-05-07 — xetex-linebreak.c: line_break

**Goal:** Translate the remaining compiled `line_break` body to Rust and finish
`xetex-linebreak.c` as a compiled-C-free file.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 78

**Work done:**
- Translated `line_break` into `src/linebreak.rs`, including pass setup,
  paragraph shape and hanging-indent handling, active-width scanning, glue and
  discretionary break handling, native-word hyphenation preparation, best-break
  selection, final-pass retry, last-line-fit adjustment, and active/passive
  cleanup.
- Added the remaining Rust global accessors needed by `line_break`, including
  `trie_not_ready`, `hyph_start`, `hyph_index`, active/background setters,
  threshold/final-pass/easy-line setters, and last-line-fit width setters.
- Disabled the C `line_break` body with `#if 0`, leaving it and the old
  `try_break` block as temporary reference material only. `xetex-linebreak.c`
  now has zero compiled C function bodies.
- Corrected three parity issues found during verification: `hang_after` is
  integer parameter 41, not 7; a non-letter inside a ligature must take WEB's
  `done3` path rather than aborting hyphenation; and `NATIVE_NODE_font` is
  `mem[p + 4].b16.s2`, not `b32.s0`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 79

**Surprises / gotchas:**
- `cargo build -p tectonic_engine_xetex` alone did not prove output parity;
  TRIP caught the ligature hyphenation control-flow mismatch and
  `tex-outputs` caught the native-node font accessor mistake.
- The disabled C reference blocks now trigger unused-static warnings in
  `xetex-linebreak.c`. They are harmless but should be deleted in a cleanup
  session.

**For the next session:**
- Delete the disabled `line_break`/`try_break` reference blocks from
  `xetex-linebreak.c`, or move on to the next in-scope C file after confirming
  the worktree state.

## Session 78 — 2026-05-07 — xetex-linebreak.c: try_break

**Goal:** Translate `try_break` to Rust and keep C `line_break` calling the
Rust implementation through the existing symbol.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 77

**Work done:**
- Added Rust accessors for the active-list scratch globals surfaced in session
  77, including `passive`, `cur_active_width`, `break_width`, best-place and
  demerit arrays, `active_node_size`, `fill_width`, and last-line-fit scratch.
- Translated `try_break` into `src/linebreak.rs`, including discretionary
  breakpoint width calculation, active/delta node insertion and removal,
  demerit calculation, and e-TeX `last_line_fit` bookkeeping.
- Disabled the C `try_break` body with `#if 0` so C `line_break` now links to
  the Rust `try_break` symbol. The disabled body remains as temporary reference
  material until `line_break` is migrated or the block is deleted.
- Fixed a control-flow bug caught by `etrip_test`: successful `last_line_fit`
  branches in C `goto found` and skip the later shortfall/glue reset. The Rust
  version now preserves that label behavior.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 78

**Surprises / gotchas:**
- The common output goldens and `engine_rust` passed before the `last_line_fit`
  fix; eTRIP was the only suite that exposed the missing finite `\parfillskip`
  adjustment. Keep eTRIP in the tight loop for the remaining line-break core.
- `small_number` is `short` in `xetex-xetexd.h`; the active-node-size extern
  should remain `i16`.
- Test failure diagnosis created `trip_etrip.log.expected` and
  `trip_etrip.log.observed` at the repository root.

**For the next session:**
- Translate `line_break`, then remove the disabled C `try_break` reference
  block and mark `xetex-linebreak.c` done if parity holds.

## Session 77 — 2026-05-07 — xetex-linebreak.c: try_break ABI prep

**Goal:** Prepare the remaining line-break core so `try_break` can be
translated without duplicating C file-static state.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 76

**Work done:**
- Verified the dirty session-76 handoff builds and all four suites still pass.
- Made `try_break` extern-visible in `xetex-linebreak.c` while keeping the C
  body in place.
- Made the active-list scratch variables used by `try_break` link-visible:
  `passive`, `cur_active_width`, `background`, `break_width`, `best_place`,
  `best_pl_line`, `disc_width`, `final_pass`, `threshold`,
  `minimal_demerits`, `minimum_demerits`, `easy_line`, `do_last_line_fit`,
  `active_node_size`, `fill_width`, `best_pl_short`, and `best_pl_glue`.
- No Rust replacement was installed for `try_break`; this was a
  no-behavior-change prep step.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 77

**Surprises / gotchas:**
- `try_break` cannot be translated in isolation while those values remain C
  file-statics, because C `line_break` initializes them and Rust would
  otherwise see a separate copy or no symbol.
- A later Rust `try_break` should add the globals accessors at the same time as
  the Rust body, so unused accessor warnings do not pile up.
- The worktree still contains unrelated generated output artifacts and
  pre-existing shipout/font accessor edits in `src/globals.rs`.

**For the next session:**
- Translate `try_break` using the now link-visible scratch state, or translate
  `line_break` and `try_break` together if you want to eliminate the bridge
  immediately.

---

## Session 76 — 2026-05-04 — xetex-linebreak.c: bottom helpers through post_line_break

**Goal:** Continue `xetex-linebreak.c` by moving the remaining bounded helpers
before the large paragraph-breaking core.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 75

**Work done:**
- Translated `get_native_usv`, `total_pw`, `find_protchar_left`,
  `find_protchar_right`, `finite_shrink`, `reconstitute`, `hyphenate`, and
  `post_line_break` to Rust in `src/linebreak.rs`.
- Added minimal global accessors for linebreak protrusion state, hyphenation
  scratch state, and the paragraph line-width/best-break state read by
  `post_line_break`.
- Deleted the corresponding C helper bodies and converted their prototypes to
  extern-visible declarations.
- Fixed a sentinel mismatch caught by `trip`: Rust `linebreak.rs` must use
  `TEX_NULL = -0x0FFF_FFFF`, not the rule-node `NULL_FLAG`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 76

**Surprises / gotchas:**
- `reconstitute` is exercised by TRIP even when common output goldens pass; keep
  TRIP in the tight loop for any future linebreak helper work.
- `post_line_break` reaches into the active-list/passive-list layout but only
  reads a subset of the line-break file statics. `try_break` and `line_break`
  share the much larger active-list scratch set and should be moved with that
  state surfaced deliberately.
- The worktree still contains unrelated dirty generated output files and
  pre-existing `globals.rs` shipout/font accessor edits from the interrupted
  prior session.

**For the next session:**
- Finish `xetex-linebreak.c`: remaining C bodies are `line_break` and
  `try_break`. Next target should be `try_break`; expose the active-list scratch
  state (`passive`, `cur_active_width`, `background`, `break_width`,
  `best_place`, `minimal_demerits`, `threshold`, `active_node_size`,
  `fill_width`, etc.) before deleting the C body.

## Session 75 — 2026-05-04 — xetex-linebreak.c: hlist stack helpers

**Goal:** Start `xetex-linebreak.c` with a small low-risk helper cluster.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 74

**Work done:**
- Added hlist-stack globals accessors for `hlist_stack` and `hlist_stack_level`.
- Created `src/linebreak.rs` and registered `mod linebreak;`.
- Translated `push_node` and `pop_node` from `xetex-linebreak.c` to Rust.
- Converted their C prototypes from `static` to extern-visible declarations and
  deleted the C bodies so existing C callers link to the Rust symbols.
- Updated `MODULE_STATUS.md` and `PROGRESS.md`.
- Commit: 1b49c054

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 75

**Surprises / gotchas:**
- The worktree already contained unrelated dirty generated output files plus
  shipout-oriented `globals.rs` accessors from a prior interrupted session.
  The commit staged only the hlist-stack portions of `globals.rs`; the unrelated
  dirty state remains for the next session or the user to handle.

**For the next session:**
- Continue `xetex-linebreak.c`. Good candidates are the smaller bottom helpers:
  `find_protchar_left`, `find_protchar_right`, or `total_pw`. Each uses linebreak
  node/accessor macros, so read the C body and add only the minimal globals/macros
  needed for that function.

## Session 74 — 2026-05-04 — xetex-pagebuilder.c: all 5 functions — file DONE

**Goal:** Translate all 5 functions from xetex-pagebuilder.c to Rust and mark the file DONE.

**Starting state:**
- Parity: 29/29
- Streak: 73

**Work done:**
- Added 6 missing globals.rs accessors: `set_page_contents`, `set_last_glue`,
  `set_last_penalty`, `set_last_kern`, `set_last_node_type`, `semantic_pagination_enabled` getter.
- Created `src/pagebuilder.rs` (916 lines) with 4 file-statics (BEST_PAGE_BREAK,
  BEST_SIZE, LEAST_PAGE_COST, PAGE_MAX_DEPTH), all constants, extern "C" block,
  and 5 translated functions: `initialize_pagebuilder_variables`,
  `freeze_page_specs`, `ensure_vbox`, `fire_up`, `build_page`.
- Registered `mod pagebuilder;` in `src/lib.rs`.
- Stubbed `xetex-pagebuilder.c` to 12-line header-only file.
- Commit: f5dbfff0

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 74

**Surprises / gotchas:**
- `as T < rhs` Rust parse ambiguity: `mem_b16s1(page_tail()) as i32 < MATH_NODE`
  was parsed as `as (i32 < MATH_NODE)` (generic). Fixed with parens.
- `fire_up`'s while loop uses the "reset p to prev_p" trick after unlinking an
  INS_NODE; translated using a `reset_p` bool flag.
- `SPLIT_UP` as NODE_type for PAGE_INS_NODE = 1, same slot as VLIST_NODE but
  different semantic context; must compare via `mem_b16s1(r) as i32 == SPLIT_UP`.
- `build_page` goto structure (contribute/update_heights/done1/done) mapped to
  boolean flags (need_contribute, need_update_heights, need_done1) with a single
  'outer: loop.

**For the next session:**
- Next target: `xetex-linebreak.c` (2843 LOC, NOT STARTED) — largest remaining file.
  Alternatively `xetex-shipout.c` (2451 LOC) if linebreak is too large for one session.

---

## Session 73 — 2026-05-04 — xetex-io.c: translate remaining 5 functions — file DONE

**Goal:** Translate the 5 deferred functions in xetex-io.c (set_input_file_encoding,
buffer_overflow, conversion_error, apply_normalization, input_line) and mark the file DONE.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- engine_rust: 7/7
- latex_goldens: 4/4
- Streak: 72
- xetex-io.c: 289 LOC, u_close already migrated but MODULE_STATUS not updated

**Work done:**
- Fixed MODULE_STATUS: marked u_close as MIGRATED (25da31e5), correct LOC/fn counts.
  (d34db49e)
- Added `buffer_ptr(offset)` accessor to `src/globals.rs` for raw buffer pointer access.
- Added three ICU C wrappers to xetex-io.c:
  - `xetex_ucnv_open` (for set_input_file_encoding)
  - `xetex_ucnv_to_utf32` (for input_line ICU path)
  (`xetex_ucnv_close` was already present for u_close)
- Declared TECkit API (`TECkit_CreateConverter`, `TECkit_ConvertBuffer`,
  `TECkit_ResetConverter`) as `extern "C"` in Rust (TECkit C++ engine has C ABI).
- Moved C file-statics to Rust: `BYTE_BUFFER`, `UTF32_BUF`, `NORMALIZERS`.
- Translated 5 functions to `src/io.rs`:
  - `set_input_file_encoding` (#[no_mangle] extern "C")
  - `buffer_overflow` (private Rust fn, was static)
  - `conversion_error` (private Rust fn, was static)
  - `apply_normalization` (private Rust fn, was static; uses TECkit)
  - `input_line` (#[no_mangle] extern "C"; ICU path + UTF-8/16/RAW path + norm)
- Removed all 5 C implementations; xetex-io.c now 101 LOC.
- MODULE_STATUS: xetex-io.c → DONE (7th file done), IN PROGRESS drops from 2 to 1.
  (725c7a39)
- All 4 suites verified: 29/29 tex-outputs, 2/2 TRIP, 7/7 engine_rust, 4/4 latex_goldens.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 73
- xetex-io.c: 101 LOC (DONE — only ICU wrappers and lookup tables remain in C)

**Surprises / gotchas:**
- apply_normalization's `static TECkit_Converter normalizers[2]` translated to Rust
  `static mut NORMALIZERS: [*mut c_void; 2]`. The `&mut NORMALIZERS[i]` pattern triggers
  a `static_mut_refs` warning (mutable reference to mutable static); this is pre-existing
  in the codebase (texmfmp.rs has the same pattern). Not blocking.
- input_line's errno check (`errno != EINTR`) uses `*libc::__errno_location()` which is
  Linux-specific but acceptable (xetex-macos.c is excluded per plan).
- `_tt_abort` is non-variadic in Rust; the normalizer error message drops the status code
  (not critical — failure is already fatal).

**For the next session:**
- xetex-io.c is DONE. Only `xetex-engine-interface.c` remains IN PROGRESS (2 fns in C
  permanently: setjmp + aarch64 linker hack).
- Natural next target: xetex-ini.c (3881 LOC, NOT STARTED) — the engine initialization
  file. Or continue with smaller NOT STARTED files (xetex-pagebuilder.c 804 LOC first).

---

## Session 72 — 2026-05-04 — xetex-xetex0.c cleanup and DONE

**Goal:** Strip the 6793-line dead-code file down to a stub and mark xetex-xetex0.c DONE.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 71
- xetex-xetex0.c: 6793 LOC (all functions behind #if 0, functionally empty)

**Work done:**
- Replaced xetex-xetex0.c with a 10-line stub: license header + two includes. All 36 `#if 0` reference blocks and their dead macros removed.
- Updated MODULE_STATUS.md: xetex-xetex0.c → DONE (6th file done), IN PROGRESS count drops from 3 to 2.
- All 4 suites verified: 29/29 tex-outputs, 2/2 TRIP.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 72
- xetex-xetex0.c: 10 LOC (DONE)

**Surprises / gotchas:**
- None. Pure cleanup.

**For the next session:**
- xetex-io.c is IN PROGRESS at 73% (3 remaining functions). That's the natural next target.

---

## Session 71 — 2026-05-03 — main_control (big_switch/reswitch dispatcher)

**Goal:** Translate the `main_control()` top-level dispatcher function into Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 70
- xetex-xetex0.c: 7065 LOC

**Work done:**
- Surveyed all command/mode constants and the C switch structure.
- Wrote `main_control()` in Rust (~270 lines) in `src/scan.rs`:
  - `'big_switch` / `'reswitch` labeled loop structure replicates the C goto network.
  - First match: fast HMODE char path → `main_control_main_loop()`; HMODE+NO_BOUNDARY → `cancel_boundary`; all before the inter-char check.
  - `main_control_hmode_inter_char_check()` returns true → `break 'reswitch` (→ big_switch).
  - Second match: full routing table of (mode, cmd) pairs to each helper function. Special cases: `IGNORE_SPACES` → `continue 'reswitch`; `VMODE+STOP` → `if its_all_over() { return; }`.
  - Mode-independent assignments (TOKS_REGISTER..EXTENSION, AFTER_ASSIGNMENT/GROUP, etc.) handled with `_ if matches!(cmd, ...)` guard.
- Replaced the 272-line C function body with a one-line comment in xetex-xetex0.c.
- All 4 suites verified: 29/29 tex-outputs, 2/2 TRIP.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 71
- xetex-xetex0.c: 6793 LOC

**Surprises / gotchas:**
- Rust match requires exhaustive pattern enumeration for the (mode, cmd) dispatch table. Used explicit tuple pairs throughout for clarity. Mode-independent commands use a `_ if matches!` guard since all three modes apply.

**For the next session:**
- xetex-xetex0.c now has only 1 remaining function stub (the comment for `main_control` that's been moved). The next logical step is to identify what remains in the ~6793 LOC and either translate or mark as done.
- The HMODE `do_locale_linebreaks` loop in `main_control_main_loop` has a TODO for the multi-iteration case.

---

## Session 70 — 2026-05-04 — main_control_main_loop (main_loop + lig_loop)

**Goal:** Translate the `main_loop` + `main_lig_loop` block (~874 lines) into Rust and wire it in.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 69
- xetex-xetex0.c: 7935 LOC

**Work done:**
- Session started mid-implementation: the Rust function `main_control_main_loop` had been written (1160 lines uncommitted) but not wired in, and had compilation errors (missing `TEX_NULL` import, missing `is_char_node` helpers).
- Fixed compilation: added `TEX_NULL` to the `use crate::mem::` import, added local `is_char_node` helpers in `main_control_main_loop` and `lig_inner_loop`.
- Wired the function into C: added `bool main_control_main_loop(void)` prototype to `xetex-xetexd.h`, replaced the 874-line `main_loop:` body with a 4-line call site.
- Fixed the `lig_result=2` wrapup bug: the wrapup guard was `lig_result != 1` (running wrapup for both result 0 and result 2), but result 2 (type-0/4 ligature with non-null lig_stack) should also skip wrapup. Changed to `lig_result == 0`. Root cause: type-0 'fi' ligature was running wrapup early, emitting an extra 'i' in the XDV output.
- Fixed `lig_result == 0xFF` sentinel: returns `continue 'char_proc` to re-enter the lig loop top without triggering wrapup or move.
- All 4 suites verified: 29/29 tex-outputs, 2/2 TRIP, 21 engine_rust, 4 latex_goldens. (commit 6f90f400)

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- `engine_rust`: 21/21
- `latex_goldens`: 4/4
- Streak: 70
- xetex-xetex0.c: 7065 LOC

**Surprises / gotchas:**
- The `lig_result != 1` wrapup guard was subtly wrong: lig_result=2 (type-0/4 lig with stack) also needs to skip wrapup (C uses `goto main_loop_move_1` which skips both wrapup and the lig_stack==TEX_NULL check). Caught by `xetex_g_builtins` test (fi-ligature in "fina'Terminal" feature name).
- The HMODE `do_locale_linebreaks` loop is still incomplete (TODO placeholder); it works for the simple no-hyphen path because the else-branch calls `do_locale_linebreaks` once before breaking.

**For the next session:**
- Only `main_control` and its dispatcher remain in C (~7065 LOC). The top-level `main_control` switch and the `big_switch` loop are the remaining pieces.
- The HMODE `do_locale_linebreaks` loop in `main_control_main_loop` has a TODO for the multi-iteration case; this should be verified/completed.

---

## Session 69 — 2026-05-02 — main_control math/spacer/ignore_spaces/inter-char cases

**Goal:** Extract the remaining large `main_control` switch-arm clusters — math cases, spacer/EX_SPACE, IGNORE_SPACES, and the HMODE inter-char pre-dispatch check — into Rust helpers.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 68
- xetex-xetex0.c: 8105 LOC

**Work done:**
- Committed Session 68 work that was uncommitted at session start (58840072).
- Added `insert_src_special_every_vbox` raw global and getter in `src/globals.rs`.
- Translated all HMODE+MATH_SHIFT through MMODE+MATH_SHIFT switch arms (~147 C lines) into `src/scan.rs::main_control_math_cases`.
- Translated HMODE+SPACER / HMODE+EX_SPACE / MMODE+EX_SPACE arms into `src/scan.rs::main_control_spacer_cases`.
- Translated `ANY_MODE(IGNORE_SPACES)` arm into `src/scan.rs::main_control_ignore_spaces`; caller retains `goto reswitch`.
- Translated the 25-line HMODE inter-char token injection block (before the main switch) into `src/scan.rs::main_control_hmode_inter_char_check() -> bool`; caller does `if (…) goto big_switch`.
- Added four prototypes to `xetex/xetex-xetexd.h`.
- Replaced C arm bodies with single Rust call sites in `xetex/xetex-xetex0.c`.
- All 4 suites verified after each commit: 29/29 tex-outputs, 2/2 TRIP, 7/7 engine_rust, 4/4 latex_goldens.
- Updated `MODULE_STATUS.md` (xetex-xetex0.c 8105→7935 LOC, last updated session 69).
- Commits: 58840072 (session 68), 4 subsequent commits for the four extracted helpers.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- `engine_rust`: 7/7
- `latex_goldens`: 4/4
- Streak: 69
- xetex-xetex0.c: 7935 LOC

**Surprises / gotchas:**
- Session 68 work was uncommitted at session start; had to commit it first.
- The Edit tool collided on a non-unique tail string in scan.rs (the closing `}` appeared 6 times); fixed by extending the match context.

**For the next session:**
- The remaining untranslated block in `main_control` is `main_loop` / `main_lig_loop` (~550 lines, goto-heavy character-processing inner loop).
- Approach: study the goto graph carefully; convert gotos to labeled `loop` + `break`/`continue` as in prior sessions.

---

## Session 68 — 2026-05-01 — main_control switch clusters

**Goal:** Make a larger dent in `main_control` by extracting switch-arm clusters with a single fall-through point.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 67
- xetex-xetex0.c: `main_control` remains in C, with one extracted label.

**Work done:**
- Cleaned generated parity artifacts from the previous failed run and kept the useful `src/globals.rs` main-loop accessor additions.
- Added the `after_token` raw global binding and setter needed by `\afterassignment`.
- Extracted the mode-independent `main_control` cluster (xetex-xetex0.c:7277) into `src/scan.rs::main_control_mode_independent`, including prefixed-command cases, `AFTER_ASSIGNMENT`, `AFTER_GROUP`, `IN_STREAM`, `MESSAGE`, `CASE_SHIFT`, `XRAY`, and `EXTENSION`.
- Extracted the forbidden/math-mode mismatch clusters into `main_control_mode_error_cases`.
- Extracted simple rule/glue/kern/group cases into `main_control_simple_list_group_cases`.
- Extracted box/list construction cases into `main_control_box_list_cases`, including the movement, paragraph-entry, mark/penalty/remove/unpackage, italic correction, discretionary, accent, and alignment-error arms.
- Extracted alignment/endcsname cases into `main_control_alignment_cases`, including the small TeXXeT `\valign` branch that appends a math node.
- Replaced the C switch-arm bodies with Rust calls and added prototypes to `xetex-xetexd.h`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- `engine_rust`: 7/7
- `latex_goldens`: 4/4
- Streak: 68
- `main_control` still owns the dispatcher; five substantial switch-arm clusters now run through Rust helpers.
- `xetex-xetex0.c`: 8105 LOC.

**Surprises / gotchas:**
- The worktree already had uncommitted `src/globals.rs` main-loop accessor additions and generated parity artifacts when the session began. Generated artifacts were cleaned; the globals were kept.
- Batching these clusters worked because every extracted arm returns to the same C `break`/`big_switch` continuation. The math-mode body below this point is more stateful and should be split more carefully.

**For the next session:**
- Continue with `main_control`, but prefer structurally coherent chunks: the math-shift/eqno entry cases and then the MMODE math-list construction cases.
- Keep the top-level dispatcher in C until the label-heavy inner loop and math cases have been reduced enough to translate control flow cleanly.

---

## Session 67 — 2026-05-01 — main_control append_normal_space

**Goal:** Start reducing `main_control` risk by translating a small self-contained label block.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 66
- xetex-xetex0.c: ~2 C functions remaining.

**Work done:**
- Ran baseline before edits: `tex-outputs` 29/29, TRIP 2/2, `engine_rust` 7/7, `latex_goldens` 4/4.
- Translated the `main_control` `append_normal_space` label (xetex-xetex0.c:8221) -> `src/scan.rs::main_control_append_normal_space` (e9988a37).
- Added Rust accessors for XeTeX inter-character spacing globals: `space_class` and `prev_class`.
- Replaced the C label body with a call to Rust followed by the original `goto big_switch`.
- Updated `MODULE_STATUS.md` and `PROGRESS.md` for session 67.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- `engine_rust`: 7/7
- `latex_goldens`: 4/4
- Streak: 67
- xetex-xetex0.c: `main_control` remains in C, but the normal-space append block is now Rust.

**Surprises / gotchas:**
- Translating all of `main_control` at once is too risky; the label-level extraction is safe because every path in the original block returns to `big_switch`.
- The inter-character token path needs to restore the current token with `back_input()` before beginning the inter-character token list, matching the C order exactly.

**For the next session:**
- Continue `main_control` by identifying another label or switch arm whose control flow returns to a single known point. Avoid a whole-function rewrite until the remaining dispatcher is smaller.

---

## Session 66 — 2026-05-01 — read_font_info

**Goal:** Translate `read_font_info` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 65
- xetex-xetex0.c: ~3 C functions remaining.

**Work done:**
- Ran baseline before edits: `tex-outputs` 29/29, TRIP 2/2, `engine_rust` 7/7, `latex_goldens` 4/4.
- Translated `read_font_info` (xetex-xetex0.c:6350) -> `src/scan.rs` (78da31d7).
- Added safe Rust accessors for TFM font-table globals used by the loader: boundary chars, lig/kern/exten bases, and `font_info[k].b16`.
- Replaced the C body in `xetex/xetex-xetex0.c` with the migration marker so callers link to Rust.
- Updated `MODULE_STATUS.md` and `PROGRESS.md` for session 66.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- `engine_rust`: 7/7
- `latex_goldens`: 4/4
- Streak: 66
- xetex-xetex0.c: ~2 C functions remaining.

**Surprises / gotchas:**
- `nine_bits` is `int32_t`, not `u16`; `font_false_bchar[f] = TOO_BIG_CHAR` needs to preserve the 65536 sentinel.
- Removing `read_font_info` from C must also remove the standalone `internal_font_number` return-type line, or it corrupts the following `main_control` declaration.
- The TFM reader intentionally mirrors C's partial EOF checking in `READFIFTEEN` and the first font parameter path.

**For the next session:**
- `main_control` is now the obvious remaining target in `xetex-xetex0.c`; it is huge and should be approached with extra care, likely by first mapping its switch cases and dependencies.

---

## Session 65 — 2026-05-01 — load_native_font

**Goal:** Translate `load_native_font` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 64
- xetex-xetex0.c: ~4 C functions remaining.

**Work done:**
- Translated `load_native_font` (xetex-xetex0.c:6284) → `src/scan.rs` (bd70d52b).
- Added raw globals and accessors for the font tables initialized by native font loading: `font_check`, `font_used`, `font_flags`, `font_letter_space`, `loaded_font_*`, and `native_font_type_flag`.
- Replaced the C body in `xetex/xetex-xetex0.c` with a Rust-linking stub comment.
- Verified all 4 suites: tex-outputs 29/29, TRIP 2/2, engine_rust 7/7, latex_goldens 4/4.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 65
- xetex-xetex0.c: ~3 C functions remaining.

**Surprises / gotchas:**
- C `mfree()` is a header-only wrapper, so Rust frees native glyph-info with `libc::free()` and explicitly stores a null pointer before `free_node()`.
- The function depends on layout-engine symbols from `tectonic_xetex_layout` plus native-font globals owned by C; accessor coverage was the main work besides the direct translation.

**For the next session:**
- Next obvious target is `read_font_info` (currently around xetex-xetex0.c:6483). `main_control` should remain last.

---

## Session 64 — 2026-05-01 — new_native_character

**Goal:** Translate `new_native_character` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 63
- xetex-xetex0.c: ~5 C functions remaining.

**Work done:**
- Translated `new_native_character` (xetex-xetex0.c:6276) → `src/scan.rs` (ea82f5de).
- Added a `mapped_text` raw global plus `mapped_text_entry()` accessor in `src/globals.rs` for the `apply_mapping()` output buffer.
- Removed the C body from `xetex/xetex-xetex0.c`, leaving the existing header prototype to link to Rust.
- Verified all 4 suites: tex-outputs 29/29, TRIP 2/2, engine_rust 7/7, latex_goldens 4/4.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 64
- xetex-xetex0.c: ~4 C functions remaining.

**Surprises / gotchas:**
- `apply_mapping` writes into the global `mapped_text` buffer owned by `xetex-ext.c`; Rust only needs an indexed read accessor.
- `measure_native_node` needs `mem_node_addr(p)`, not `mem_ptr(p)`, matching the prior `do_locale_linebreaks` gotcha.

**For the next session:**
- Next obvious targets are `load_native_font` (line 6360) and then `read_font_info` (line 6559). `main_control` should remain last.

---

## Session 63 — 2026-05-01 — print_cmd_chr, new_native_word_node, do_locale_linebreaks

**Goal:** Translate `print_cmd_chr` (the large command-display switch), `new_native_word_node`, and `do_locale_linebreaks` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 62
- xetex-xetex0.c: ~9 C functions remaining.

**Work done:**
- Removed `fn print_cmd_chr(cmd: u16, chr_code: i32);` from extern "C" blocks in scan.rs, errors.rs, and input.rs; added `use crate::output::print_cmd_chr;` to each.
- Added ~250 lines of `CMD_*` / `CC_*` constants to output.rs covering all command codes and sub-codes.
- Translated `print_cmd_chr` (line 1265, ~1400 lines) → `src/output.rs` (5184bb4d). Match on `u16` cmd; CALL/OUTER_CALL tracing via `n % 2` odd-test; REGISTER case with local `(the_cmd, the_chr)` binding; SET_FONT font-area flag comparison; XETEX_MATH_GIVEN bit extraction inlined.
- Wrapped `print_cmd_chr` C body in `#if 0 / #endif`.
- Translated `new_native_word_node` (line 6250, ~22 lines) → `src/scan.rs` (64b6b451). Node size computed as `NATIVE_NODE_SIZE + (n * 2 + 7) / 8`.
- Translated `do_locale_linebreaks` (line 6493, ~55 lines) → `src/scan.rs` (64b6b451). Calls `linebreak_start`/`linebreak_next` via local extern block; `main_f`/`native_text` accessed as extern statics.
- Wrapped both C bodies in `#if 0 / #endif`.
- All 4 test suites pass (29+2+7+4 = 42).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 63
- xetex-xetex0.c: ~7 C functions remaining.

**Surprises / gotchas:**
- `print_cmd_chr` REGISTER case locally reassigns `cmd` in C; modelled as `(the_cmd, the_chr)` local tuple in Rust since you can't rebind a match variable.
- `CC_AAT_FONT_FLAG = 0xFFFF` and `CC_OTGR_FONT_FLAG = 0xFFFE` must be `i32` to match `font_area_entry()` return type.
- `mem_node_addr(p)` (pointer TO mem[p]) vs `mem_ptr(p)` (pointer FROM mem[p].ptr field): `measure_native_node` always needs `mem_node_addr`.
- `do_locale_linebreaks`'s `main_f`/`native_text` are not in globals.rs; accessed via a local `extern "C"` block in scan.rs.

**For the next session:**
- `new_native_character` (line 6274, ~75 lines): uses `apply_mapping`, `char_warning`, `map_char_to_glyph`, font-mapping globals.
- `load_native_font` (line 6358, ~130 lines): font engine APIs.
- `read_font_info` (~6555, very large TFM reader).
- `main_control` (~7427, the huge main engine loop).

---

## Session 62 — 2026-05-01 — scan_glyph_number, round_decimals, xetex_scan_dimen

**Goal:** Translate `scan_glyph_number`, `round_decimals`, and `xetex_scan_dimen` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 61
- xetex-xetex0.c: ~10 C functions remaining.

**Work done:**
- Translated `scan_glyph_number`, `round_decimals`, `xetex_scan_dimen` → `src/scan.rs` (7058f857).
- Wrapped all three C bodies in `#if 0 / #endif`.
- All 4 test suites pass.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 62
- xetex-xetex0.c: ~9 C functions remaining.

**Surprises / gotchas:**
- None significant; these were straightforward scanner functions.

**For the next session:**
- `print_cmd_chr` (line 1265, ~1400 lines): large display switch.
- `new_native_word_node`, `do_locale_linebreaks`, `new_native_character`, `load_native_font`, `read_font_info`, `main_control`.

---

## Session 61 — 2026-04-30 — get_next, macro_call

**Goal:** Translate `get_next` and `macro_call` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 60
- xetex-xetex0.c: ~12 C functions remaining.

**Work done:**
- Translated `get_next` (~490 lines, xetex-xetex0.c:3167) → `src/scan.rs` (97dbabde).
  - Added `used_tectonic_coda_tokens` global to globals.rs.
  - Complex nested labeled loops: `'restart`, `'texswitch`, `'reswitch`, `'start_cs`.
  - CRITICAL bug found and fixed: COMMENT case (cat=7) missing `set_cur_input_loc(limit+1)`.
    Symptoms: XDV 96+ bytes larger, trip test `\toks0` instead of `\output`.
    Fix: `15 | 31 | 47 | 22 => { set_cur_input_loc(cur_input().limit + 1); continue 'texswitch; }`
- Translated `macro_call` (~330 lines, xetex-xetex0.c:3665) → `src/scan.rs` (6c98b044).
  - Added `param_size` accessor to globals.rs.
  - `pstack` translated as a local `[i32; 9]` array (was C global but only used in macro_call).
  - goto-label structure: `'mac:` block for exit cleanup, `'outer:` loop for do-while,
    `'continue_:` loop for inner token-reading cycle.
- All 4 test suites pass (29+2+7+4 = 42).
- Updated SESSION_LOG.md, MODULE_STATUS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 61
- xetex-xetex0.c: ~10 C functions remaining.

**Surprises / gotchas:**
- COMMENT case bug: easy to miss `cur_input.loc = cur_input.limit + 1` since it looks like a goto-only effect. Without it, `%` comments process rest of commented line as real tokens.
- `pstack` is a C global but logically local to macro_call; translating as local array avoids adding another global.

**For the next session:**
- `print_cmd_chr` (line 1265, ~1500 lines): very large display function with CALL/OUTER_CALL tracing.
- After that: `scan_glyph_number`, `round_decimals`, `xetex_scan_dimen`, `new_native_*`, `load_native_font`, `do_locale_linebreaks`, `read_font_info`, `main_control`.

---

## Session 60 — 2026-04-30 — expand

**Goal:** Finish `expand` translation (compilation errors remained from session 59) and commit.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 59
- xetex-xetex0.c: ~13 C functions remaining.

**Work done:**
- Fixed `expand_depth_count += 1` → `set_expand_depth_count(expand_depth_count() + 1)` (getter, not variable).
- Fixed `expand_depth_count >= expand_depth()` call form.
- Fixed `print_cmd_chr(cur_cmd(), ...)` → `print_cmd_chr(cur_cmd() as u16, ...)` (u8→u16 cast).
- Fixed `print_cmd_chr(FI_OR_ELSE_CMD, ...)` → `print_cmd_chr(FI_OR_ELSE_CMD as u16, ...)`.
- Wrapped C `expand` body in `#if 0 / #endif`.
- All 4 test suites pass. Committed as b01234a8.
- Updated SESSION_LOG.md, MODULE_STATUS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 60
- xetex-xetex0.c: ~12 C functions remaining.

**Surprises / gotchas:**
- `expand_depth_count` is a getter function — bare `+= 1` fails at compile time with "binary assignment operation cannot be applied to fn item". Must always call setter explicitly.
- `cur_cmd()` returns `u8` but `print_cmd_chr` expects `u16`; must cast at every call site.

**For the next session:**
- `get_next` (line 3164, ~495 lines): core token reader, very large.
- `macro_call` (line 3660, ~339 lines): macro expansion.
- `print_cmd_chr` (line 1265, ~1500 lines): very large display function.
- After those: `scan_glyph_number`, `round_decimals`, `xetex_scan_dimen`, `new_native_*`, `load_native_font`, `do_locale_linebreaks`, `read_font_info`, `main_control`.

---

## Session 59 — 2026-04-30 — show_context, expand (partial)

**Goal:** Translate `show_context` and `expand` from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 58
- xetex-xetex0.c: ~14 C functions remaining.

**Work done:**
- Added `trick_buf(k)` read accessor to globals.rs (was missing; only set_trick_buf existed).
- Added `force_eof` global static and `force_eof()`/`set_force_eof()` accessors to globals.rs.
- Added `set_cur_order()` setter to globals.rs.
- Translated `show_context` (line 2764, ~232 lines) → `src/scan.rs` (64464240). C `goto done` → `'outer: loop { ... break 'outer; }`.
- Wrapped `show_context` C body in `#if 0 / #endif`.
- All 4 test suites pass after show_context.
- Translated `expand` (~272 lines) → `src/scan.rs`. Hit compilation errors left for session 60.
- Removed `fn expand()` from extern "C" block in scan.rs (duplicate symbol).
- Added `name_in_progress` to module-level imports in scan.rs.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 59 (show_context committed; expand pending)
- xetex-xetex0.c: ~13 C functions remaining.

**Surprises / gotchas:**
- Removing from `extern "C"` block required when the Rust fn is defined in the same crate.

---

## Session 58 — 2026-04-29 — flush_node_list, copy_node_list, show_activities, show_node_list, print_subsidiary_data

**Goal:** Translate node-list management functions and display functions from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 57

**Work done:**
- Translated `flush_node_list` (line 754, ~182 lines) → `src/scan.rs` (9dbfdea7). Recursive node freeing with `'done` labeled block.
- Translated `copy_node_list` (line 954, ~160 lines) → `src/scan.rs` (0fc07566). Allocates fresh node chain; NATIVE_WORD_NODE inner loop sets `words=0` so outer copy loop is skipped.
- Added `nest_head`, `nest_mode_line`, `nest_aux_b32s0` accessors to globals.rs.
- Translated `show_activities` (line 1123, ~122 lines) → `src/scan.rs` (947f0a58).
- Made `cur_length()` in hash.rs `pub(crate)`.
- Translated `print_subsidiary_data` (line 215, ~37 lines) and `show_node_list` (line 259, ~470 lines) → `src/scan.rs` (a5e002a6).
- Updated PROGRESS.md, MODULE_STATUS.md, SESSION_LOG.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 58
- xetex-xetex0.c: ~14 C functions remaining.

**Surprises / gotchas:**
- `#if 0` must go BEFORE the return type in C, not inside the function body — caught after first build attempt on `flush_node_list`.
- `NATIVE_WORD_NODE` in `copy_node_list` sets `words=0` via its own inner loop, so the outer copy loop does nothing after it. Must translate faithfully.

**For the next session:**
- `show_context` (line 2764, ~232 lines): error context display, medium complexity.
- `get_next` (line 3164, ~495 lines): core token reader, very large.
- `macro_call` (line 3660, ~339 lines): macro expansion.
- `expand` (line 4000, ~289 lines): expansion dispatcher.
- `print_cmd_chr` (line 1265, ~1500 lines): very large display function.
- After those: `scan_glyph_number`, `round_decimals`, `xetex_scan_dimen`, `new_native_*`, `load_native_font`, `do_locale_linebreaks`, `read_font_info`, `main_control`.

## Session 57 — 2026-04-29 — conv_toks, begin_token_list

**Goal:** Translate `conv_toks` (string-conversion token builder) and `begin_token_list` (input-stack token-list pusher).

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 56

**Work done:**
- Added extern "C" declarations for `aat_print_font_name`, `gr_print_font_name`, `print_glyph_name` to scan.rs.
- Translated `conv_toks` (5162, ~350 lines) → `src/scan.rs` (719c7946). Two-phase switch: Phase 1 computes `c`/`cur_val`; Phase 2 builds the string; `str_toks_cat` finishes. `PDF_CREATION_DATE_CODE` calls `begin_token_list` in Phase 1 then falls through Phase 2 (which emits an empty token list, immediately consumed).
- Wrapped `conv_toks` C body in `#if 0 / #endif`.
- Added constants `OUTPUT_TEXT`, `MARK_TEXT`, `WRITE_TEXT`, `INT_BASE_BTL`, `INT_PAR_TRACING_MACROS`, `LOCAL_BASE_BTL`, `LOCAL__OUTPUT_ROUTINE_BTL`, `ASSIGN_TOKS_BTL` to input.rs.
- Added extern "C" declarations for `print_cmd_chr`, `capture_to_diagnostic` to input.rs.
- Translated `begin_token_list` (2970, ~50 lines) → `src/input.rs` (73ff1645).
- Wrapped `begin_token_list` C body in `#if 0 / #endif`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 57
- xetex-xetex0.c: ~17 C functions remaining.

**Surprises / gotchas:**
- `ASSIGN_TOKS` must be 73, not 72. `TOKS_REGISTER = 72`, `ASSIGN_TOKS = 73` (xetex_format.h). Using 72 caused TRIP test failure where token registers printed as `\toks0` instead of `\output`, `\everydisplay`, etc.
- `aat_print_font_name` on Linux: `CFDictionaryRef` is `typedef void*` in the non-Mac header, so declare as `*mut libc::c_void` in Rust — safe.

**For the next session:**
- `show_token_list` (line 20, ~83 lines): needs `first_count` and `half_error_line` globals added to globals.rs first.
- `check_outer_validity` (line 3031, ~90 lines): medium, no missing globals expected.
- `flush_node_list` (line 754, ~182 lines): recursive, complex node traversal.

---

## Session 56 — 2026-04-28 — vpackage, hpack (+ SIGSEGV fix)

**Goal:** Translate `vpackage` and `hpack`, the vertical/horizontal box packers.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 55

**Work done:**
- Translated `vpackage` (7424, ~220 lines) with `'glue: {}` labeled-block for goto-common_ending/goto-exit pattern → scan.rs (79e7c7fd).
- Translated `hpack` (6972, ~450 lines) with the same glue-block pattern, bidi LR_ptr stack, NATIVE_WORD_NODE merging logic, raw u16 pointer text copy, INS/MARK/ADJUST redirection.
- Fixed SIGSEGV in `hpack` NATIVE_WORD_NODE path: `measure_native_node` takes `&mem[p]` (= `mem_node_addr(p)`), not `mem[p].ptr` (= `mem_ptr(p)`). The C macro `set_native_metrics(p,g)` expands to `measure_native_node(&mem[p], g)`.
- Fixed same bug in `do_extension` GLYPH_CODE path: `measure_native_glyph` also expects `mem_node_addr`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 56
- xetex-xetex0.c: ~19 C functions remaining.

**Surprises / gotchas:**
- `mem_ptr(p)` vs `mem_node_addr(p)`: `mem_ptr(p)` reads the `ptr` union field of `mem[p]` (the raw pointer stored IN the word), while `mem_node_addr(p)` returns a pointer TO `mem[p]`. All `set_native_metrics`/`set_native_glyph_metrics` C macros use `&mem[p]`, so always use `mem_node_addr`.
- The OTF SIGSEGV was traced to the wrong pointer being passed to `measure_native_node`; it was reading bogus memory from the `ptr` union field.

**For the next session:**
- `conv_toks` (5162): medium-large (~350 lines), string conversion for `\the` commands.
- `show_node_list` and related display functions may also be candidates.

---

## Session 55 — 2026-04-28 — close_files_and_terminate, prune_page_top, do_marks, do_extension

**Goal:** Translate the file-termination function, page-top pruning, mark-tracking recursion, and the whatsit extension dispatcher.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 54

**Work done:**
- Added `write_open[18]` global + accessor to globals.rs; added setters for `random_seed`, `epochseconds`, `microseconds`.
- Added extern "C" declarations for `ttstub_output_close`, `ttstub_output_putc`, `finalize_dvi_file`, `synctex_terminate`, `out_what`, `load_picture`, `measure_native_glyph`, `get_encoding_mode_and_info` to scan.rs.
- Added `call_set_input_file_encoding` wrapper in io.rs to avoid clashing-extern-declarations warning.
- Translated `close_files_and_terminate` (9722, ~40 lines) → scan.rs (5830f81c).
- Translated `prune_page_top` (9761, ~70 lines) → scan.rs (5830f81c).
- Translated `do_marks` (9817, ~80 lines, recursive tree traversal) → scan.rs (5830f81c).
- Translated `do_extension` (8024, ~165 lines, 13 switch cases) → scan.rs (bc4e9176).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 55
- xetex-xetex0.c: ~21 C functions remaining.

**Surprises / gotchas:**
- `do_extension` GLYPH_CODE case: `set_native_glyph_metrics(p, g)` is a macro that expands to `measure_native_glyph(&mem[p], g)`. Used `mem_ptr(cur_list_tail())` to get the raw pointer.
- `do_marks` recursive: Rust calls itself directly. The `cur_ptr` global is used as a traversal cursor between recursive levels (matches the C exactly).
- `close_files_and_terminate` uses `write_open[18]` (18-element array, indices 0–15 used here). Added accessor.
- `prune_page_top` calls `new_skip_param(GLUE_PAR__split_top_skip)` which sets `temp_ptr` as a side effect; the function then reads `mem[temp_ptr + 1].b32.s1` to adjust the glue stretch.

**For the next session:**
- `hpack` (6972): large (~450 lines), the horizontal packer — complex but well-defined.
- `vpackage` (7424): large (~220 lines).
- `conv_toks` (5162): medium-large (~350 lines), string conversion for `\the` commands.

---

## Session 54 — 2026-04-27 — just_copy, just_reverse, handle_right_brace, char_pw

**Goal:** Translate the bidi node-copying functions, the right-brace handler, and char_pw.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 53

**Work done:**
- Added `LR_ptr`, `LR_problems`, `cur_dir`, `last_leftmost_char`, `last_rightmost_char` globals + accessors.
- Added `set_page_tail`, `set_output_active`, `nest_tail`, `set_nest_tail` accessors to globals.rs.
- Added extern "C" declarations for `do_marks`, `prune_page_top`, `ship_out`, `fin_mlist`, `build_choices`, `real_get_native_word_cp`.
- Translated `just_copy` (8029, ~125 lines) → `src/scan.rs` (a6f7231b). Goto found/not_found modelled as `Option<i32>` from the match arm.
- Translated `just_reverse` (8155, ~95 lines) → `src/scan.rs` (a6f7231b). Inner `goto done` modelled as `break 'done` on labeled loop.
- Translated `handle_right_brace` (8265, ~215 lines, 16 group cases) → `src/scan.rs` (b6d9eca8).
- Translated `char_pw` (6968, ~45 lines) → `src/scan.rs` (978de7d8). Uses `mem_node_addr` to pass `&mem[p]` to `real_get_native_word_cp`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 54
- xetex-xetex0.c: ~25 C functions remaining.

**Surprises / gotchas:**
- `just_copy` LIGATURE_NODE case: `goto found` skips the word-copy loop. Modelled by setting `words = 0` before returning `Some(r)`.
- `just_reverse`: line 8242 `goto done` after the while loop means the 3 lines after the loop (8243-8245) are dead code — never executed. They were omitted in Rust.
- `char_pw`: `CHAR_NODE_font(p)` = `mem[p].b16.s1` (not `.b16.s3` as one might expect — the "type/font" field of a char node is stored in s1).
- `handle_right_brace` MATH_GROUP: `mem[save_stack[save_ptr].b32.s1].b32 = mem[p+1].b32` — this copies both b32 fields (s0 and s1) at once. In Rust, split into two `set_mem_b32s0/s1` calls.

**For the next session:**
- `do_extension` (8068): medium — whatsit handlers.
- `prune_page_top` (9805): medium, already has extern "C" declared.
- `do_marks` (9861): medium, already has extern "C" declared.
- `close_files_and_terminate` (9766): medium.

---

## Session 53 — 2026-04-27 — vert_break, vsplit, box_end, begin_box

**Goal:** Translate several medium-complexity functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 52

**Work done:**
- Added `active_width[7]` and `best_height_plus_depth` globals + accessors to globals.rs.
- Translated `vert_break` (7725, ~135 lines) → `src/scan.rs` (0d1c6960). Goto tangle (not_found/update_heights/done) modelled as a `Next` enum + labeled `'done: loop`.
- Added `set_cur_mark` accessor; added `do_marks` and `prune_page_top` extern "C" declarations; added `set_eqtb_b32s1` to the scan.rs import list.
- Translated `vsplit` (7727, ~102 lines) → `src/scan.rs` (9c2a6546). while+goto done → simple `loop { ...; break; }`.
- Added `ship_out` extern "C" declaration; removed old local `extern "C" { fn box_end }` declaration from `scan_box`.
- Translated `box_end` (7797, ~90 lines) → `src/scan.rs` (32d30dd9).
- Translated `begin_box` (7794, ~180 lines) → `src/scan.rs` (bf5b0f45). `goto done` in LAST_BOX_CODE modelled as `'last_box_done: { ... break 'last_box_done; }` Rust labeled block.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 53
- xetex-xetex0.c: ~29 C functions remaining.

**Surprises / gotchas:**
- `active_width` is 1-indexed in C (7-element array, indices 1-6 used). In Rust, passed as `active_width(1)` through `active_width(6)`.
- `begin_box` LAST_BOX_CODE has `goto done` inside a do-while loop. Modelled as a Rust labeled block `'last_box_done: { ... break 'last_box_done; }` — requires Rust 1.65+ labeled block breaks (project already uses Rust 1.77+).
- `begin_box` default case: when `k` is VTOP_GROUP, it sets `k = VMODE` after the scan_spec call. This was modelled as `k_mode` computed post-branch.

**For the next session:**
- `just_copy` (8124): ~125 lines, bidi node copying.
- `just_reverse` (8250): ~130 lines, bidi node reversal.
- `handle_right_brace` (8578): very large — many group cases (~400 lines).

---

## Session 52 — 2026-04-26 — show_save_groups + do_assignments

**Goal:** Two more functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 51

**Work done:**
- Added `nest_e_tex_aux(k)` accessor to globals.rs.
- Added `set_box_allowed` global + `set_set_box_allowed` accessor to globals.rs.
- Translated `show_save_groups` (7752, ~210 lines) → `src/scan.rs` (0963a923).
- Translated `do_assignments` (10745, 14 lines) → `src/scan.rs` (04e4888a).
- Removed two local `extern "C"` declarations for `show_save_groups` and `do_assignments`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 52
- xetex-xetex0.c: ~33 C functions remaining.

**Surprises / gotchas:**
- `show_save_groups` has a complex goto pattern (found/found1/found2/done) with fallthrough. Modelled as `enum After { Preamble, Found1, Found2, Found }` with chained conditional blocks.
- C `do-while { m = nest[p].mode; if p>0 p--; else m=VMODE; } while m==HMODE`: in Rust, read m first, conditionally decrement p, then break when m!=HMODE. After the loop, `p` is one below the entry that stopped the loop, which is the intended behavior.
- `set_box_allowed` is a C global that needed explicit extern linking (xetex-ini.c defines it).

**For the next session:**
- `show_info` (7717): probably 5-10 lines.
- `vert_break` (7754): ~135 lines, moderate complexity.
- `handle_right_brace` (8843): very large.

## Session 51 — 2026-04-26 — fin_align

**Goal:** Translate fin_align from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 50

**Work done:**
- Added `pack_begin_line` global and `cur_list_mode_line()` accessor to globals.rs.
- Added `resume_after_display` to the top-level extern "C" block in scan.rs.
- Translated `fin_align` (8040, ~300 lines) → `src/scan.rs` (97f42b9a).
- Removed local `extern "C" { fn fin_align(); }` from `align_peek`.
- Also: restored the lost session 50 translations (init_align/fin_col/fin_row) that were wiped by a git stash/checkout during the hotfix investigation; committed as 59a8e644.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 51
- xetex-xetex0.c: ~35 C functions remaining.

**Surprises / gotchas:**
- `set_mem_b32s1(DIMEN_BASE + ...)` silently writes way past the mem[] array bounds (mem has ~5M slots; DIMEN_BASE ≈ 8.9M). Should have been `set_eqtb_b32s1`. TRIP test caught the SIGSEGV immediately — this is exactly why the TRIP suite is mandatory at every step.
- The session 50 translations (init_align/fin_col/fin_row) were accidentally lost during the hotfix investigation: `git stash; git checkout f4f6bf8c -- src/; git stash pop` reverted the committed translations. They were re-applied in commit 59a8e644.
- `fin_align` is the most complex function translated so far (~300 lines, 3 nested loops, glue ratio computation, MMODE/VMODE branching).

**For the next session:**
- `handle_right_brace` (9973): very large — many group cases.
- `show_save_groups` (8350): medium, diagnostic helper.
- `just_copy` / `just_reverse` (9427/9428): bidi node copying, complex.

## Session 50b — 2026-04-26 — hotfix: append_italic_correction seg-fault

**Goal:** Fix the SIGSEGV in the LaTeX smoke test that session 50 exposed.

**Root cause:**
The session 46 translation of `append_italic_correction` called
`mem_ptr(tail)` to get the node pointer for `real_get_native_italic_correction`.
`mem_ptr(p)` reads `mem[p].ptr` — the VALUE of the ptr union field — which
for a NATIVE_WORD_NODE at word 0 is the type/subtype/link bytes, NOT a valid
pointer.  The correct call is the C macro `get_native_italic_correction(p)`
= `real_get_native_italic_correction(&mem[p])` — the ADDRESS of word 0.

The bug was latent through sessions 46–49 because the execution path
(hmode processing after an alignment) happened to hit a NATIVE_WORD_NODE
as the tail only after session 50 altered the memory layout.  On a 64-bit
host, the garbage ptr had upper bytes = TEX_NULL = 0xF0000001, making it
an invalid kernel-space address → SIGSEGV.

**Fix:** Added `mem_node_addr(p) -> *mut c_void` = `&mem[p]` to globals.rs
and switched both NATIVE_WORD_NODE and GLYPH_NODE branches to use it (2e219d67).

**Ending state:**
- Smoke test: PASS
- Parity: 29/29
- Trip: 2/2
- Streak: 50 (maintained)

## Session 50 — 2026-04-26 — alignment triad: init_align / fin_col / fin_row

**Goal:** Translate the three core alignment functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 49

**Work done:**
- Added `adjust_tail`, `pre_adjust_tail`, `total_stretch[4]`, `total_shrink[4]` to globals.rs (needed by fin_col).
- Added `nest_aux_b32s1(k)` accessor to globals.rs (needed by init_align).
- Added `flush_math` to the top-level extern "C" block in scan.rs (called by init_align).
- Translated `fin_col` (7859), `fin_row` (8009), `init_align` (7732) → `src/scan.rs` (724e76bb).
- Removed local `extern "C"` declarations for `fin_col`/`fin_row` from `do_endv` (now same-module calls).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 50
- xetex-xetex0.c: ~38 C functions remaining.

**Surprises / gotchas:**
- `fin_col` reuses local variable `q` for two distinct purposes (first = preamble node after cur_align, then = HOLD_HEAD for list building). Used `wq` as the second usage.
- `cur_tail` in C is the alignment `cur_tail` (Rust: `cur_tail_a()`), distinct from `cur_list.tail`. Easy to confuse.
- `init_align` uses `nest[nest_ptr - 2].aux.b32.s1` — needed a new `nest_aux_b32s1(k)` accessor in globals.rs.
- Function-level `use` blocks must explicitly import every `crate::globals` function used; module-level imports don't extend into function bodies with local use blocks that shadow them for the same symbols.

**For the next session:**
- `fin_align` (8040): ~160 lines. Handles the column width adjustment pass after all rows are done. Uses hpack/vpackage + complex span arithmetic.
- `handle_right_brace` (9973): very large — many group cases.

## Session 49 — 2026-04-26 — get_preamble_token

**Goal:** One more function from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 48

**Work done:**
- Translated `get_preamble_token` (7728) → `src/scan.rs` (236b6293). goto-restart pattern as labeled `'restart` loop.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 49
- xetex-xetex0.c: ~41 C functions remaining.

**Surprises / gotchas:**
- `get_token` and `scan_glue` are in the same module; call directly without `crate::scan::` prefix.

**For the next session:**
- `fin_col` (7924): complex, ~100 lines.
- `fin_row` (8114): medium.
- `init_align` (7756): medium, uses push_alignment, scan_spec.

## Session 48 — 2026-04-26 — math/scan/alignment cluster

**Goal:** Continue translating functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 47

**Work done:**
- Translated `scan_math` (4313), `set_math_char` (4315) → `src/scan.rs` (d90d1986, 38f8512d). Shared `math_{char,class,fam}/set_{class,family}` inline helpers.
- Translated `new_character` (7048) → `src/scan.rs` (e05d388f).
- Translated `scan_glue` (4678) → `src/scan.rs` (04720330). Added `cur_order` to globals.rs.
- Translated `push_alignment` (7724), `pop_alignment` (7743) → `src/scan.rs` (57961939). Added alignment globals.
- Translated `init_span` (7880), `init_row` (7893), `init_col` (7912) → `src/scan.rs` (300e358f).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 48
- ~9 new functions this session.
- xetex-xetex0.c: ~11594 total C LOC, ~42 C function signatures remaining.

**Surprises / gotchas:**
- `scan_math` has two nested gotos (restart/reswitch): modelled as a `'main` loop containing a `loop { break match { ... } }` block.
- `scan_glue` was in the extern "C" block — must be removed before defining the Rust function.
- `init_row` row-mode toggle is `-105 - cur_list.mode` which flips between inner-hmode (-104) and inner-vmode (-1).
- `cur_tail` (alignment) conflicts with nothing but was named `cur_tail_a` in Rust accessors to avoid confusion with `cur_list_tail`.

**For the next session:**
- `fin_col` (7924): complex, 100+ lines. Handles column termination in alignments.
- `fin_row` (8114): calls fin_col helpers, medium.
- `get_preamble_token` (7768): medium, preamble scanning.
- `scan_font_ident` (4335, inside #if 0 — already translated? No, still in C).
- `char_pw` (7053): uses complex font metric macros.

## Session 47 — 2026-04-26 — file/box/graf/space cluster

**Goal:** Continue translating functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 46

**Work done:**
- Added `gave_char_warning_help`, `main_p`, `main_k`, `cur_lang`, `insert_src_special_every_par`, `cur_box` globals to `globals.rs`.
- Translated `char_warning` (6428), `app_space` (9223) → `src/scan.rs` (a7098003).
- Translated `new_graf` (9549) → `src/scan.rs` (6732bacf).
- Translated `package` (9509) → `src/scan.rs` (c283130c). Added `hpack`/`vpackage`/`box_end` to top-level extern block.
- Translated `insert_src_special` (9978) → `src/scan.rs` (2ee99a3c).
- Translated `scan_box` (9485) → `src/scan.rs` (5ffb71e6).
- Translated `tt_insert_special` (9984) → `src/scan.rs` (9e355d5b).
- Translated `pack_file_name` (6381) → `src/scan.rs` (feb6bd59). Added `set_name_of_file` to `globals.rs`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 47
- ~8 new functions this session.
- xetex-xetex0.c: ~11878 total C LOC, ~51 C function signatures remaining.

**Surprises / gotchas:**
- `ttstub_issue_warning` is a variadic C function; declared with `...` in the Rust extern block (works on Rust 1.90).
- `char_warning` writes to `INTPAR(tracing_online)` (an eqtb cell), which requires `set_eqtb_b32s1`.
- `insert_src_special` was already being called from `new_graf` (via extern C), so removing from extern block after Rust translation was required.
- `pack_file_name` was in both the top-level extern block AND a local extern block — needed to remove both.

**For the next session:**
- `scan_math` (4313): medium, math scanning. Uses many math-specific globals.
- `char_warning` callers still in C; `insert_src_special_every_vbox` / `insert_src_special_auto` patterns similar.
- `new_character` (7200): creates char nodes. Medium complexity.
- `handle_right_brace` (10018): very large, complex — many group cases.
- `extra_right_brace` (9278, #if 0): already in Rust at `src/scan.rs:2079`.
- `just_copy`/`just_reverse` (9627/9658): bidi node copying — complex.

## Session 46 — 2026-04-25 — accent/italic/unpackage cluster

**Goal:** Continue translating functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 45

**Work done:**
- Translated `make_accent` (9910) → `src/scan.rs` (3a0adf77).
- Translated `append_italic_correction` (9868) → `src/scan.rs` (53f45e8b).
  - Gotcha: `get_native_italic_correction` / `get_native_glyph_italic_correction` are C macros that expand to `real_get_native_italic_correction(&mem[p])` — must call the `real_` prefixed functions with `mem_ptr(p)`.
- Translated `delete_last` (9716), `unpackage` (9799) → `src/scan.rs` (e5313fe2).
  - Added `disc_ptr` accessor to `globals.rs`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 46
- ~300 Rust functions total; ~6 new this session.
- xetex-xetex0.c: ~31244 total C LOC, ~80 C function signatures remaining.

**Surprises / gotchas:**
- `get_native_italic_correction(p)` is a C macro: `real_get_native_italic_correction(&mem[p])`. In Rust, call `real_get_native_italic_correction(crate::globals::mem_ptr(p))`.

**For the next session:**
- `app_space` (9223): needs `main_p`/`main_k` globals (add to globals.rs), uses font_glue.
- `package` (9624): medium, \hbox/\vbox packaging.
- `scan_math` (4313): medium, math scanning.
- `char_warning` (6428): medium, lost character warning.
- `just_copy` (9992): complex node copying.

---

## Session 45 — 2026-04-25 — vlist/alignment/show/register/font cluster

**Goal:** Continue translating functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 44

**Work done:**
- Translated `append_to_vlist` (8020), `align_peek` (8785) → `src/scan.rs` (c2b751b9).
- Translated `show_whatever` (10561) → `src/scan.rs` (b8e223bc).
- Translated `do_register_command` (10220) → `src/scan.rs` (177ee736).
- Translated `new_font` (10233) → `src/scan.rs` (e44c3dce); added `font_size_entry`/`font_dsize_entry` to `globals.rs`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 45
- ~281 Rust functions total; ~6 new this session (plus 2 private helpers in globals).
- xetex-xetex0.c: ~31494 total C LOC, ~65 C functions remaining in xetex-xetex0.c.

**Surprises / gotchas:**
- `show_whatever` has two `goto common_ending` jumps; modelled as a boolean `goto_common_ending` flag — clean and readable.
- `do_register_command` reuses the `q` variable as both a command code and a glue spec node pointer in the ADVANCE glue branch; used a separate `qs` variable in Rust to avoid confusion.
- `new_font` has two-strategy font lookup: first by name+area (native fonts), then by full file path (TFM). Both strategies checked inside the same loop.

**For the next session:**
- `make_accent` (10087): complex, font metrics + kern arithmetic.
- `append_to_vlist` dependents are now all in Rust.
- Remaining ~65 functions in xetex-xetex0.c include `do_extension`, `main_control`, and several pagebuilder/linebreak helpers still in C.

---

## Session 44 — 2026-04-24 — discretionary + message cluster

**Goal:** Continue translating small functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 43

**Work done:**
- Translated `shift_case` (10765), `build_discretionary` (9958) → `src/scan.rs` (9bd0a3da).
- Added `long_help_seen`, `set_long_help_seen`, `set_use_err_help` to `globals.rs`.
- Translated `issue_message` (10708) → `src/scan.rs` (c9d261aa).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 44
- 286 Rust functions total; 3 new this session.
- xetex-xetex0.c: 12936 LOC, ~73 C functions remaining.

**Surprises / gotchas:**
- `build_discretionary` had a node-type condition bug: the C nests `!= KERN_NODE` and `!= LIGATURE_NODE` inside `> RULE_NODE` (> 2). Translating the whole condition as `> KERN_NODE` (> 11) silently accepted GLUE nodes (type 10), missing the "Improper discretionary list" error. The TRIP test caught this via a diff in trip.log.

**For the next session:**
- `show_whatever` (10793): medium, dispatches \show/\showbox/\showlists/\showifs. Depends on `show_save_groups`, `show_activities` (still in C), `cond_ptr`, `if_line`, `cur_if`.
- `do_register_command` (10439): complex goto-heavy function, ~200 LOC.
- `make_accent` (10087): complex, font metrics + kern arithmetic.
- `new_font` (10642): very long, ~130 LOC.
- `append_to_vlist` (8020): short, needs `prev_depth` check.
- `align_peek` (8785): short alignment helper.

---

## Session 43 — 2026-04-24 — main_control helper cluster

**Goal:** Continue translating small functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 42

**Work done:**
- Added `par_loc`, `par_token` accessors to `globals.rs`.
- Translated `off_save` (9325), `indent_in_hmode` (9808), `head_for_vmode` (9831), `append_src_special` (11376) → `src/scan.rs` (a38b0b96).
- Translated `tokens_to_string` (13060), `compare_strings` (13073), `new_write_whatsit` (11045), `begin_insert_or_adjust` (9760) → `src/scan.rs` (d09daa78).
- Translated `do_endv` (10161), `get_r_token` (10407) → `src/scan.rs` (b80b0aba).
- Translated `append_discretionary` (9956) → `src/scan.rs` (b0c95ff1).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 43
- 283 Rust functions total; 11 new this session.
- xetex-xetex0.c: 13118 LOC, ~76 C functions remaining.

**Surprises / gotchas:**
- `head_for_vmode` had a heap-corruption bug: passing the return value of `error_here_with_diagnostic` back to `capture_to_diagnostic` caused a double-free. `error_here_with_diagnostic` already calls `capture_to_diagnostic(error)` internally, so the caller must always pass `null_mut()` — never the returned pointer.
- Binary search (enabling/disabling functions one at a time) was needed to isolate this bug. 4 functions → 3 binary search steps.

**For the next session:**
- `build_discretionary` (9982): depends on `unsave` (in Rust), `copy_node_list` (C). Medium complexity.
- `do_register_command` (10439): complex with goto → needs `gsa_w_def`, `sa_w_def`, `gsa_def`, `sa_def` (C extern). Long function.
- `issue_message` (10774): medium, uses `token_show`, `flush_list`, `overflow`, `use_err_help`, `long_help_seen`.
- `shift_case` (10831): medium, scans toks and modifies case codes.
- `show_whatever` (10859): medium, dispatcher for \show/\showbox/etc.
- `new_font` (10642): very long (~130 LOC), complex.
- `append_to_vlist` (8020): depends on `prev_depth` in output.rs; check if needed.

---

## Session 42 — 2026-04-23 — continued small function cluster

**Goal:** Continue translating small functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 41

**Work done:**
- Added `page_tail`, `set_dead_cycles`, `set_insert_penalties`, `set_page_so_far`, `set_cur_list_aux_b32s0`, `set_nest_prev_graf` to `globals.rs`.
- Translated `box_error` (9270), `its_all_over` (9317), `append_glue` (9358), `append_kern` (9395) → `src/scan.rs` (b2c2f894).
- Translated `make_mark` (9896), `new_interaction` (11040) → `src/scan.rs` (1669e578).
- Translated `alter_aux` (10774), `alter_prev_graf` (10805), `alter_page_so_far` (10837), `alter_integer` (10846), `alter_box_dimen` (10874) → `src/scan.rs` (49b91f62).
- Translated `trap_zero_glue` (10570), `fix_language` (11357) → `src/scan.rs` (94b086c4).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 42
- 272 Rust functions total; 13 new this session.
- xetex-xetex0.c: 13398 LOC, ~87 C functions remaining.

**Surprises / gotchas:**
- `BOX_BASE` was already defined at top of scan.rs; got a duplicate-constant error adding it again near box_error. Same for `MU_GLUE`. Check for existing constants before adding new ones.
- LaTeX smoke test PDF size varied by 2 bytes between runs due to PDF timestamp metadata — not a behavioral regression.
- `alter_prev_graf` uses `nest[p].mode` in a while-loop condition; used `nest_mode(p).unsigned_abs()` in Rust since `i16::abs()` is available but the comparison to `VMODE` (u16) needs `.unsigned_abs() as u16`.

**For the next session:**
- `show_save_groups` (8829): complex, many gotos — defer.
- `off_save` (9367): uses get_avail, back_input, error handling and multi-case switch.
- `do_endv` (10471): calls end_graf, fin_col, fin_row, off_save — depends on fin_col/fin_row still in C.
- `indent_in_hmode` (9808): uses new_null_box, new_noad, mem writes.
- `head_for_vmode` (9831): calls off_save (still in C).
- `append_src_special` (11401): synctex helper — check dependencies.

---

## Session 41 — 2026-04-23 — small function cluster

**Goal:** Translate a large batch of small, self-contained functions from `xetex-xetex0.c`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 40

**Work done:**
- Added `font_mapping` and `xtx_ligature_present` globals to `globals.rs`.
- Added `cur_list_e_tex_aux()`, `set_cur_list_aux_b32s1()` accessors to `globals.rs`.
- Translated `effective_char` (4457) + `effective_char_info` (6434) → `src/scan.rs` (223bdfc2).
- Translated `eTeX_enabled` (8811) → `src/scan.rs`, `print_totals` (9282) → `src/output.rs` (3521e31f).
- Translated `you_cant` (9326), `report_illegal_case` (9334), `privileged` (9347) cluster → `src/scan.rs` (11e31397). Caught u8→u16 widening bug in `print_cmd_chr` call.
- Translated `normal_paragraph` (9496) → `src/scan.rs` (862f5198).
- Translated `give_err_help` (13499) → `src/errors.rs` (removed extern decl from that module), `new_whatsit` (13763) → `src/mem.rs` (e8430054).
- Translated `end_graf` (9942) + `append_penalty` (10016) → `src/scan.rs` (4af3b2b8).
- Translated `align_error` (10390), `no_align_error` (10443), `omit_error` (10457), `cs_error` (10489) → `src/scan.rs` (5f06879e).
- Translated `push_math` (10416) → `src/scan.rs` (650e6dd0).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 41
- 259 Rust functions total; 17 new this session.
- xetex-xetex0.c: 13641 LOC, ~100 C functions remaining.

**Surprises / gotchas:**
- `effective_char` was already declared in scan.rs's extern block (used by scan_something_internal); needed to remove the extern decl when translating.
- `you_cant` calls `print_cmd_chr(cur_cmd, cur_chr)` where C silently widens `uint8_t cur_cmd` to `uint16_t`. Rust requires explicit `as u16` cast.
- `new_whatsit` had its return type (`void`) on a separate line; accidentally left it behind when deleting the body, causing a C parse error. Fixed by including the `void` line in the deletion.
- `print_if_line` mentioned in prior session notes does not exist as a standalone function in the current C source.

**For the next session:**
- `show_save_groups` (8829): complex multi-goto diagnostic — defer.
- `its_all_over` (9357): calls `build_page` + `back_input` + uses `mem` writes.
- `do_endv` (10471): calls `fin_col`, `fin_row`, `off_save` — still in C.
- `box_error` (9311): calls `show_box`, `flush_node_list` — check dependencies.
- `append_glue` (9383), `append_kern` (9418): direct mem writes + scan helpers.

---

## Session 40 — 2026-04-23 — start_input

**Goal:** Translate `start_input` (the primary input-file opener, ~160 lines), the natural next step recommended by session 39.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 39

**Work done:**
- Added `input_file_ptr(k)` accessor to `globals.rs` (returns `*mut *mut UFile`, analogous to existing `read_file_ptr`).
- Added `mod raw_utf8` with `bytesFromUTF8`/`offsetsFromUTF8` extern statics to `scan.rs` for the UTF-8 decode loop in the primary-input-name path.
- Added `INT_PAR_XETEX_DEFAULT_INPUT_MODE = 75`, `INT_PAR_XETEX_DEFAULT_INPUT_ENCODING = 76`, `INT_PAR_TRACING_STACK_LEVELS = 55`, `TTBC_FILE_FORMAT_TEX_SI = 26`, `TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI = 59` constants.
- Added `synctex_start_input` to `extern "C"` block in `scan.rs`.
- Translated `start_input` (6431) to `src/scan.rs`.
- First test run: `pipe_input` test failed — expected error message to include filename but we called `_tt_abort(c"failed to open input file")` without the name. Fixed by formatting the message with `libc::snprintf` into a stack buffer first.
- Deleted C body with tombstone comment.
- Committed as `54064e99`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 40
- 242 Rust functions total; 1 new this session.
- xetex-xetex0.c: 13867 LOC, ~117 C functions remaining.

**Surprises / gotchas:**
- `_tt_abort` is declared non-variadic in Rust, but the C code passes `name_of_file` as a format arg. The `pipe_input` test checks for the filename in the error message, so the message must include it. Used `libc::snprintf` into a stack buffer to pre-format the message.
- `input_file` in globals is a `*mut *mut c_void` (heap pointer), not an array. Need `.offset(k)` not `.as_mut_ptr().add(k)` to get `&input_file[k]`.

**For the next session:**
- `show_save_groups` (~10952 in current file): complex multi-goto diagnostic helper — defer if possible.
- `effective_char_info` (~6595) and `char_info` — typed-view helpers used by font lookup code.
- `print_if_line` (~3475): short diagnostic helper.
- Consider the cluster around `effective_char` and font char info (lines ~6595+).

---

## Session 39 — 2026-04-22 — conditional

**Goal:** Translate `conditional` (the `\if`/`\ifcase`/`\ifx`/… expansion machine), a contained ~400-line function that has been the natural next step since `pass_text` and `scan_something_internal` both landed in Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 38

**Work done:**
- Rechecked baseline after `9b03ca67`; held at 29/29 / TRIP 2/2.
- Added `is_in_csname` global (bool) to `globals.rs`.
- Translated `conditional` (6271) to `src/scan.rs`. Handled all 22 `\if…` sub-codes plus the `is_unless` inversion, the `tracing_ifs`/`tracing_commands` diagnostic paths, and the `goto common_ending` idiom.
- Extracted `pop_cond_node()` as a private `unsafe fn` (the §515 pattern repeated 6 times in C).
- First test run: both TRIP tests failed. Diff showed "Extra \fi" error from `IF_CASE_CODE`. Root cause: the `goto common_ending` inside the `IF_CASE_CODE` while-loop was implemented as an unconditional `pop_cond_node()` call, but `common_ending` actually branches on `cur_chr`: pop the node only if `cur_chr == FI_CODE`, else just set `if_limit = FI_CODE`. Fixed.
- Removed C body with tombstone comment.
- Committed as `cdd6f395`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 39
- 241 Rust functions total; 1 new this session.
- xetex-xetex0.c: 14029 LOC, ~118 C functions remaining.

**Surprises / gotchas:**
- The `goto common_ending` idiom in C requires careful mapping: `common_ending` is not just "pop a node" — it dispatches on `cur_chr == FI_CODE` vs. `ELSE_CODE`. Forgetting this made both TRIP tests fail with a spurious "Extra \fi".

**For the next session:**
- `start_input` (~7540): input file opener, ~170 lines. Depends on `open_log_file` (Rust).
- `show_save_groups` (~10952): complex multi-goto diagnostic helper.

---

## Session 38 — 2026-04-22 — scan_something_internal + scan_int

**Goal:** Translate the two central scanner functions that have been "next up" for many sessions, plus the open_log_file warmup from session 37.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 37

**Work done:**
- Rechecked baseline after `da715977`; held at 29/29 / TRIP 2/2.
- Added 26 new extern globals to `globals.rs`: page-builder state (`page_contents`, `page_so_far`, `dead_cycles`, `insert_penalties`, `output_active`, `last_penalty/kern/glue/node_type/badness`), font encoding arrays (`font_area`, `font_bc`, `font_ec`, `font_layout_engine`), font dimension bases (`char_base`, `width_base`, `height_base`, `depth_base`, `italic_base`), miscellaneous (`cur_val1`, `radix`, `random_seed`, `pdf_last_x/y_pos`). Also added ~40 accessor functions and fixed `shell_escape_enabled()` getter that was missing.
- Added `cur_list_tail()` getter, `cur_list_aux_b32s0/s1()`, `nest_mode()`, `nest_prev_graf()` accessors.
- Added `font_info_b16()`, per-font array accessors (`font_area_entry`, `font_bc_entry`, `font_ec_entry`, `font_layout_engine_ptr`, `hyphen_char_entry`, `skew_char_entry`, and the five dimension-base accessors).
- Added ~30 new `extern "C"` declarations to `scan.rs` for AAT/OT engine APIs, native-char metrics, glyph mapping, font type queries, and `new_spec`.
- Added ~165 lines of new constants to `scan.rs` covering all command codes, value types, eqtb bases, LAST_ITEM sub-codes, node codes, token constants for scan_int, etc.
- Translated `scan_something_internal` (4505) and `scan_int` (5465) to `src/scan.rs`, preserving the `goto restart` as a `'restart: loop`, all switch arms, the coerce/negate epilogue, and the full LAST_ITEM fan-out.
- Removed both C bodies with tombstone comments.
- First test run: TRIP `trip_test` failed. Diff showed `0.0pt` expected vs `20.02014pt` observed for `\showthe\fontdimen1000\trip`. Root cause: `ASSIGN_FONT_DIMEN` case used `set_mem_b32s1(fmem_ptr(), 0)` instead of `set_font_info_b32s1(fmem_ptr(), 0)` — wrote the sentinel to `mem[]` instead of `font_info[]`. Fixed; TRIP went back to 2/2.
- Committed as `9b03ca67`.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 38
- 240 Rust functions total; 2 new this session.
- xetex-xetex0.c: 14434 LOC, ~119 C functions remaining.

**Surprises / gotchas:**
- `font_info` and `mem` are two distinct arrays. In the `ASSIGN_FONT_DIMEN` case, `font_info[fmem_ptr].b32.s1 = 0` clears a sentinel in `font_info[]`, not `mem[]`. Using `set_mem_b32s1` instead of `set_font_info_b32s1` silently compiled but corrupted the wrong array — caught only by the TRIP test.
- Several constants that were scoped inside individual functions (`INT_VAL`, `DIMEN_VAL`, `FI_CODE`, `DEF_FAMILY`, etc.) conflicted with the new module-level constants block. The cleanup involved removing the redundant per-function consts. `FI_CODE` was typed as `i32` in the old location and `u8` in the new one — fixed the one usage that needed `as i32`.
- The `goto restart` in `scan_something_internal` (the IGNORE_SPACES `\primitive` dispatch) maps cleanly to `continue 'restart;` in a labeled loop.

**For the next session:**
- `conditional` (~7375): `\if` expansion machine, ~250 lines. Now feasible since both `scan_something_internal` and `pass_text` are in Rust.
- `start_input` (~7940): input file opener, ~170 lines. Depends on `open_log_file` (Rust).

---

## Session 37 — 2026-04-22 — open_log_file

**Goal:** Translate `open_log_file`, completing the filename/logging cluster now that `pack_job_name` and `make_name_string` are both in Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 36

**Work done:**
- Rechecked the baseline after `9347fe3b`: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Added `texmf_log_name` to the raw extern block in `globals.rs`, plus getters/setters for `texmf_log_name`, `log_file` (setter), `log_opened` (setter), and `job_name` (setter).
- Added `ttstub_output_open` to the `extern "C"` block in `scan.rs`.
- Translated `open_log_file` (7937) to `src/scan.rs`, preserving the `old_setting`/selector save-restore, the `job_name == 0 → "texput"` fallback, the `pack_job_name(".log")` call, the INVALID_HANDLE abort, the `input_stack[0].limit` / `end_line_char` catch-up replay, and the `old_setting + 2` selector restore.
- Removed the C body from `xetex-xetex0.c`.
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`; parity stayed 29/29 and TRIP stayed 2/2.
- Committed as `da715977`.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 37
- 238 Rust functions total; 1 new this session.
- xetex-xetex0.c: ~15537 LOC, ~121 C functions remaining.

**Surprises / gotchas:**
- `_tt_abort` in Rust is declared without varargs, so the C `_tt_abort("cannot open log file output \"%s\"", name_of_file)` becomes `_tt_abort(c"cannot open log file output".as_ptr())` — less informative but behaviorally identical (engine aborts). This is the established pattern in the codebase.
- `INT_PAR_END_LINE_CHAR` was already defined earlier in `scan.rs` (at the `read_toks` section), so the duplicate const would have caused a compile error. Removed the redundant definition from the `open_log_file` section.

**For the next session:**
- The filename/logging cluster is now complete. The main planned next target is `scan_something_internal` (~4552): large core register/parameter scanner.
- `conditional` (~7420) follows naturally after `scan_something_internal`.

---

## Session 36 — 2026-04-22 — make_name_string

**Goal:** Keep pushing the filename/logging cluster with another small helper that now sits directly on the Rust side of `pack_job_name`.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 35

**Work done:**
- Committed session 35 as `b375cb8f` (`engine_xetex: translate pack_job_name to Rust`).
- Rechecked the baseline after `b375cb8f`: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Translated `make_name_string` (7928) to `src/scan.rs`, preserving the early `'?'` overflow/partial-name exit, the temporary UTF-16 repack into the pool, and the save/restore of `area_delimiter`, `ext_delimiter`, `name_in_progress`, and `stop_at_space`.
- Removed the C body from `xetex-xetex0.c`.
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs` after the translation; parity stayed 29/29 and TRIP stayed 2/2.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 36
- 237 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15574 LOC, ~122 C functions remaining.

**Surprises / gotchas:**
- `make_name_string` looks string-pool heavy, but it turns out to compose cleanly from already-migrated pieces: `make_utf16_name`, `begin_name`, `more_name`, `end_name`, and `make_string`.
- This leaves the filename/logging area in a nicer spot for a future `open_log_file` session if we want another contained win before returning to the larger scanner work.

**For the next session:**
- `scan_something_internal` (~4552) is still the main planned target.
- If you want one more contained helper before that, `open_log_file` is now more attractive: `pack_job_name` and `make_name_string` are both already in Rust.

---

## Session 35 — 2026-04-22 — pack_job_name

**Goal:** Clear the tiny filename wrapper next to `scan_file_name` before returning to the larger scanner core.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 34

**Work done:**
- Rechecked the baseline after `47dd4f05`: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Translated `pack_job_name` (7970) to `src/scan.rs` as a direct Rust wrapper around `maketexstring`, `job_name`, and `pack_file_name`.
- Removed the C body from `xetex-xetex0.c`.
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs` after the translation; parity stayed 29/29 and TRIP stayed 2/2.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 35
- 236 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15611 LOC, ~123 C functions remaining.

**Surprises / gotchas:**
- This really was the small cleanup it looked like in the plan: no new globals were needed, because `job_name`, `cur_area`, `cur_ext`, `cur_name`, `maketexstring`, and `pack_file_name` were already reachable from Rust.
- The raw grep-based function counter for `xetex-xetex0.c` still overshoots the curated tracker, so the per-session “C functions remaining” number should keep following the existing best-effort tracker unless we do a dedicated recount pass.

**For the next session:**
- `scan_something_internal` (~4552) is now the clear next target again. The nearby filename helpers are fully out of the way.
- `conditional` (~7420) still follows naturally after `scan_something_internal`.

---

## Session 34 — 2026-04-22 — scan_file_name + braced helper

**Goal:** Continue the scanner/file-name cluster with the next self-contained pair rather than forcing `scan_something_internal` prematurely.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 33

**Work done:**
- Rechecked the baseline after the `bbf2fad1` / `e6c0c922` commits: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Translated `scan_file_name_braced` (static helper at 7967) as a private Rust helper in `src/scan.rs`.
- Translated `scan_file_name` (8003) to `src/scan.rs`, preserving the `warning_index`, `scanner_status`, `def_ref`, `cur_cs`, and `stop_at_space` save/restore flow exactly.
- Removed the C bodies from `xetex-xetex0.c`.
- Fixed one missed constant after the first compile (`OTHER_CHAR = 12` from `xetex_format.h:244`).
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs` after the translation; parity stayed 29/29 and TRIP stayed 2/2.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 34
- 235 Rust functions total; 2 new this session.
- xetex-xetex0.c: 15617 LOC, ~124 C functions remaining.

**Surprises / gotchas:**
- `scan_file_name_braced` needs the same temporary-string capture pattern used elsewhere in `scan.rs`: switch `selector` to `SELECTOR_NEW_STRING`, call `show_token_list`, then `make_string()` and clean up the token list.
- The only compile break was a missing local constant for `OTHER_CHAR`; once that was added, behavior matched the C with no parity changes.
- `scan_something_internal` still looks best treated as a dedicated session. The remaining scanner/file-name wrappers are now smaller and less entangled.

**For the next session:**
- `pack_job_name` (~7970) is a trivial nearby warm-up if you want one more tiny cleanup before `scan_something_internal`.
- Otherwise go straight back to `scan_something_internal` (~4552); with `scan_and_pack_name` and `scan_file_name` now in Rust, that switch has fewer C-side dependencies.

---

## Session 33 — 2026-04-22 — scan_and_pack_name

**Goal:** Make another incremental pass at the scanner cluster by translating a small helper that `scan_something_internal` already depends on.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 32

**Work done:**
- Re-ran the required baseline checks: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Read through `scan_something_internal` and the adjacent scanner/file-name code. Decided not to force a risky 600-line translation in one shot while several branches still rely on many C macros/globals.
- Translated `scan_and_pack_name` (13424) to `src/scan.rs` as a small, direct helper for the XeTeX-specific branches inside `scan_something_internal`.
- Removed the C body from `xetex-xetex0.c`.
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs` after the translation; parity stayed 29/29 and TRIP stayed 2/2.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 33
- 233 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15691 LOC, ~126 C functions remaining.

**Surprises / gotchas:**
- `scan_something_internal` is broad enough that translating it faithfully wants a dedicated session: it mixes register logic, page state, font/layout queries, last-node inspection, and a `goto restart` path for primitive lookup.
- `scan_and_pack_name` was a good intermediate step because it is already called from several XeTeX-only branches of `scan_something_internal`, so future work there has one less C callee to juggle.

**For the next session:**
- Return to `scan_something_internal` (~4552). A good first move is to split the translation mentally into major switch regions: code tables, token/font cases, register/page/box cases, `LAST_ITEM`, and the `IGNORE_SPACES` restart path.
- `conditional` (~7420) remains the next major follow-on once `scan_something_internal` is in Rust.

---

## Session 32 — 2026-04-22 — find_font_dimen

**Goal:** Translate the small font-parameter helper queued by session 31 before tackling the much larger scanner core.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 31

**Work done:**
- Re-ran the required baseline checks before editing: `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs`. Baseline held at parity 29/29 and TRIP 2/2.
- Added minimal font-memory accessors to `src/globals.rs`: `font_info`, `fmem_ptr`, `font_ptr`, `font_mem_size`, `font_params`, `font_glue`, and `param_base`.
- Translated `find_font_dimen` (4501) to `src/scan.rs`. The C growth loop stays literal: zero the new `font_info[fmem_ptr].b32.s1`, then increment `fmem_ptr` and `font_params[f]` until the requested slot exists.
- Removed the C body from `xetex-xetex0.c`.
- Re-ran `cargo build -p tectonic_engine_xetex`, `cargo test -p tectonic --test trip`, and `cargo test -p tectonic --test tex-outputs` after the translation; parity stayed 29/29 and TRIP stayed 2/2.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, and `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 32
- 232 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15695 LOC, ~127 C functions remaining.

**Surprises / gotchas:**
- `find_font_dimen` needed `FONT_ID_BASE + f` for the diagnostic path; `hash_s1` from `globals.rs` was enough, so no raw hash-table access had to leak into `scan.rs`.
- The helper returns `cur_val = fmem_ptr` as a sentinel in two distinct error cases (`n <= 0` and "fontdimen too late on an existing font"). Preserving that shared sentinel is what makes the final diagnostic branch line up with the C.
- This session left the pre-existing non-fatal warnings untouched; the only new warning I introduced briefly was an unused import, which was removed before finishing.

**For the next session:**
- `scan_something_internal` (~4552) is now the clear next target. It sits immediately after `find_font_dimen`, and most of its small helpers/callees are already in Rust.
- `conditional` (~7420) remains the next major follow-on once `scan_something_internal` is translated.

---

## Session 31 — 2026-04-21 — scan_toks, read_toks, filename cluster, extra_right_brace, scan_font_ident, show_info

**Goal:** Translate the \def-scanner (scan_toks), \read-scanner (read_toks), filename cluster, and small error/util functions.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 30

**Work done:**
- Translated `scan_toks` (7081) to `src/scan.rs`. goto done1/done → labeled loop returning bool; goto found → break 'found. Committed 36606974.
- Translated `read_toks` (7268) to `src/scan.rs`. do/while → loop/break. Committed 34f9603e.
- Added globals: area_delimiter, ext_delimiter, quoted_filename, file_name_quote_char, stop_at_space getter, name_in_progress getter, cur_name/area/ext setters.
- Translated `begin_name` (7827) + `more_name` (7837) + `end_name` (7870) to `src/scan.rs`. Committed 716de350.
- Translated `extra_right_brace` (11411) to `src/scan.rs`. Committed a77d4e05.
- Translated `scan_font_ident` (4467) to `src/scan.rs`. Committed 252c1cea.
- Translated `show_info` (9942) to `src/scan.rs`. Committed 72b59f43.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 31
- 231 Rust functions total; 9 new this session.
- xetex-xetex0.c: 15743 LOC, ~128 C functions remaining.

**Surprises / gotchas:**
- scan_toks had `const SPACER` already defined at module scope (from get_token area) — would-be duplicate constant had to be removed.
- `read_toks` uses `NORMAL` = 0 for scanner_status, which conflicts with existing constant names. Used `NORMAL_SCANNER` to avoid collision.
- `end_name` pool-shift loops: the C uses index arithmetic on pool arrays; Rust needs careful tracking of which `str_ptr` value is current at each step.
- `scan_font_ident` was simple once `scan_math_fam_int` was confirmed Rust; the only issue was a duplicate SPACER constant.

**For the next session:**
- `scan_something_internal` (~4552): very large (~600 LOC), the core register/parameter reader. Most callees are in Rust. This is a critical target.
- `conditional` (~7420): ~400 LOC, the \if-expansion machine. Needs `pass_text` (Rust), `change_if_limit` (Rust), `scan_something_internal` (still C).
- `find_font_dimen` (~4501): small but needs `fmem_ptr`, `font_params`, `font_glue`, `param_base` globals.

---

## Session 30 — 2026-04-21 — token/string cluster: pseudo_start, ins_the_toks, str_toks*, the_toks, pass_text

**Goal:** Translate the token-building and string-conversion cluster recommended by session 29, plus nearby small functions.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 29

**Work done:**
- Added globals: `open_parens` (xetex-xetexd.h:508), `skip_line` (xetex-xetexd.h:541). Added missing `set_mem_b16s2`/`set_mem_b16s3` setters to globals.rs.
- Translated `ins_the_toks` (6511) + `pseudo_start` (6263) to `src/scan.rs`. Committed 267f103c.
- Translated `str_toks_cat` (6363) + `str_toks` (6417) to `src/scan.rs`. Committed a89914f7.
- Translated `the_toks` (6427) to `src/scan.rs`. Committed 1643e7d9.
- Translated `pass_text` (7380) to `src/scan.rs`. goto done → loop/break. Committed 5e2cc852.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 30
- 222 Rust functions total; 6 new this session.
- xetex-xetex0.c: 15719 LOC, ~137 C functions remaining.

**Surprises / gotchas:**
- `pseudo_start` packs str_pool bytes into b16x4 words (4 u16 per node word); needed `set_mem_b16s2`/`set_mem_b16s3` which were missing from globals.rs.
- `the_toks` uses `cur_val_level()` which returns `u8`; comparing to `IDENT_VAL: i16` required casting.
- In `str_toks_cat`, the C variable `m` that tracks the position in the inner pack loop was renamed `mm` to avoid shadowing the outer `m`.

**For the next session:**
- `conditional` (~7410): large (~400 lines), the full \if-expansion machine. Most helpers are Rust; calls `scan_something_internal` (still C) and `pass_text` (now Rust).
- `read_toks` (~7266): reads from \read streams; calls `input_line` (C), `begin_file_reading` (Rust).
- `scan_toks` (~7075): the \def/\edef scanner, ~180 lines; calls `get_token`, `scan_general_text` etc. (all Rust).

---

## Session 29 — 2026-04-20 — scan_expr (complex recursive scanner)

**Goal:** Translate scan_expr, whose helpers (get_x_or_protected, scan_keyword, scan_dimen, add_or_sub, mult_and_add, quotient, fract) are all now in Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 28

**Work done:**
- Added globals: expand_depth, expand_depth_count, cur_val_level. Committed edba0a63.
- Translated `scan_expr` (6025) to `src/scan.rs`. Two backward gotos (restart, continue_) become a labeled `'outer` loop + inner loop; `goto found` (skip value-scan) becomes `skip_scan=true`. o mutation in case 3 (EXPR_MULT→EXPR_SCALE) handled by declaring `o` as `mut`. C body replaced with `#if 0` block. Committed edba0a63.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 29
- 216 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15704 LOC, ~143 C functions remaining.

**Surprises / gotchas:**
- The `o = EXPR_SCALE` mutation inside `match s { 3 => ... }` requires `o` to be declared as `let mut o` before the match — otherwise it shadows rather than mutates.
- Initial draft had post-match loop logic duplicated inside case 3; second pass moved it out.
- C body kept as `#if 0` block (not deleted) to preserve line-number references during review.

**For the next session:**
- `ins_the_toks` (~6700): trivial 1-liner wrapping `the_toks`.
- `pseudo_start` (~6266): uses scan_general_text (Rust) + str_pool direct access.

---

## Session 28 — 2026-04-20 — scan_general_text

**Goal:** Translate scan_general_text now that get_token, scan_left_brace are in Rust.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 27

**Work done:**
- Added globals: scanner_status, warning_index, def_ref (with getters/setters). Committed a19aad9a.
- Translated `scan_general_text` (6262) to `src/scan.rs`. goto found replaced with loop/break. Committed a19aad9a.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 28
- 215 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15705 LOC, ~144 C functions remaining.

**Surprises / gotchas:**
- None; clean translation.

**For the next session:**
- `pseudo_start` (~6317): uses scan_general_text (now Rust), token_show (C), str_pool/pool_ptr, get_avail/get_node.
- `scan_expr` (~6025): complex recursive; all its key helpers are now in Rust.

---

## Session 27 — 2026-04-20 — scan cluster: rule_spec, dimen, decimal, spec

**Goal:** Exploit that scan_keyword is now Rust to translate dependent scan functions.

**Starting state:**
- Parity: 29/29
- Trip: 2/2
- Streak: 26

**Work done:**
- Translated `scan_rule_spec` (6267) to `src/scan.rs`. goto reswitch → loop. Committed 96737d98.
- Translated `scan_dimen` (5956), `scan_decimal` (5961) to `src/scan.rs`. Both trivial wrappers around `xetex_scan_dimen`. Committed 6e9aea72.
- Translated `scan_spec` (9215) to `src/scan.rs`. goto found → if/else. Committed f614c4f3.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 27
- 214 Rust functions total; 4 new this session.
- xetex-xetex0.c: 15758 LOC, ~145 C functions remaining.

**Surprises / gotchas:**
- None; all translations were clean.

**For the next session:**
- `scan_expr` (~6025): complex recursive scanner; now get_x_or_protected, scan_keyword, scan_dimen all in Rust — a good target.
- `scan_general_text` (~6262): uses scanner_status, get_x_token, back_input, error (all Rust).

---

## Session 26 — 2026-04-20 — scan_keyword (short session)

**Goal:** Translate scan_keyword, whose callees are now all in Rust.

**Starting state:**
- Parity: 29/29 (tex-outputs)
- Trip: 2/2
- Streak: 25

**Work done:**
- Translated `scan_keyword` (4283) to `src/scan.rs`. Added `begin_token_list` extern "C" and `BACKUP_HEAD`/`BACKED_UP` constants. Added `back_input` local inline wrapper. Committed c8ccdb2d.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 26
- 210 Rust functions total; 1 new this session.
- xetex-xetex0.c: 15820 LOC, ~149 C functions remaining.

**Surprises / gotchas:**
- None; clean translation.

**For the next session:**
- `scan_expr` (~6227): complex recursive scanner; now all helpers (get_x_or_protected, back_input, scan_keyword) are in Rust.
- `scan_rule_spec` (~6420): medium; uses scan_keyword (now Rust), scan_dimen (C).

---

## Session 25 — 2026-04-20 — get_x_token, get_x_or_protected (short session)

**Goal:** Translate the two main token-fetching functions that are called by many other functions.

**Starting state:**
- Parity: 29/29 (tex-outputs)
- Trip: 2/2
- Streak: 24

**Work done:**
- Translated `get_x_token` (4272) and `get_x_or_protected` (4533) to `src/scan.rs`. Added constants CALL, END_TEMPLATE, ENDV, FROZEN_ENDV, PROTECTED_TOKEN. macro_call() still extern "C". Committed 42ba5a71.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 25
- 209 Rust functions total; 2 new this session.
- xetex-xetex0.c: 15875 LOC, ~150 C functions remaining.

**Surprises / gotchas:**
- `get_x_token` had an orphaned C `}` brace after replacing its body — needed a second edit pass.

**For the next session:**
- `scan_keyword` (~4284): all callees are now Rust (get_x_token, back_input, get_avail, flush_list); medium complexity.
- `scan_expr` (~6227): complex scanner; get_x_or_protected is now Rust.

---

## Session 24 — 2026-04-20 — input stack, warnings, scanner helpers (14 functions)

**Goal:** Translate begin/end_file_reading cluster and other small functions.

**Starting state:**
- Parity: 29/29 (tex-outputs)
- Trip: 2/2
- Streak: 23

**Work done:**
- Translated `begin_file_reading` (3097), `end_file_reading` (3127) to `src/input.rs`. Added globals: first, buf_size, max_in_open, source_filename_stack, eof_seen, input_file + setters for line, in_open, etc. Committed 778f6b77.
- Translated `pseudo_input` (2722) to `src/input.rs`. Added last, max_buf_stack, set_buffer_entry, set_cur_input_limit globals. Committed 061dc82a.
- Translated `bad_utf8_warning` (8970) to `src/output.rs`. Committed 4fa969cd.
- Translated `font_feature_warning` (8712), `font_mapping_warning` (8732), `graphite_warning` (8764) to `src/output.rs`. Added `print_utf8_str` extern "C" link. Committed 3802c1cf.
- Translated `x_token` (4316), `scan_optional_equals` (4355) to `src/scan.rs`. Added cur_cs global. Committed 81d5e934.
- Translated `scan_left_brace` (4319) to `src/scan.rs`. Added set_cur_cmd, set_cur_chr. Committed 01c50e13.
- Translated `norm_min` (12014) to `src/helpers.rs`, `insert_dollar_sign` (11444) to `src/scan.rs`. Committed 8440fd8c.
- Translated `insert_relax` (3998) to `src/input.rs`. Committed c6e5204d.
- Translated `get_token` (3653) to `src/scan.rs`. Added set_no_new_control_sequence. Committed e789d157.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 24
- 207 Rust functions total; 14 new this session.
- xetex-xetex0.c: 15914 LOC, ~152 C functions remaining.

**Surprises / gotchas:**
- `pseudo_input` uses `mem[r].b16` to read 4 packed u16 values; the b16 layout is s3,s2,s1,s0 low-to-high in memory. Used individual `mem_b16s0..s3` accessors to match C field mapping.
- `insert_dollar_sign` (`'$' = 36`): the token `MATH_SHIFT_TOKEN + 36` is exactly `MATH_SHIFT_TOKEN + '$' as i32` in Rust.
- `INSERTED` constant appeared twice in input.rs after reorganization — needed to deduplicate.

**For the next session:**
- `get_x_or_protected` (~4550): small (calls `get_token` + `expand`); both in Rust or extern C now.
- `scan_keyword` (~4347): medium but no new globals needed; uses `get_x_token`, `back_input`, `get_avail`, `flush_list`.
- `scan_expr` (~6227): complex recursive scanner; needs `get_x_or_protected`, `scan_something_internal`, `scan_int`, `scan_dimen` (all still in C).
- `scan_math` (~4452): depends on `get_x_token`, `eqtb` macros — needs more infrastructure.

---

## Session 23 — 2026-04-19 — plan: add TRIP test + 11 more functions

**Goal:** Add TRIP test to plan/workflow, then translate more functions.

**Starting state:**
- Parity: 29/29 (tex-outputs)
- Trip: 2/2
- Streak: 22

**Work done:**
- Updated plan docs to include `cargo test -p tectonic --test trip` in 05-testing.md, 06-workflow.md, PROGRESS.md. Fixed package name (tectonic, not tectonic_engine_xetex). Committed 0e6791d6, 967b4187.
- Translated `add_or_sub` (6281), `quotient` (6305), `fract` (6336) to `src/scaled_math.rs`. Committed 3b22036d.
- Translated `change_if_limit` (7889) to `src/output.rs`. Added `set_mem_b16s1` to output.rs imports. Committed 3b22036d.
- Translated `scan_normal_glue` (6519), `scan_mu_glue` (6524) to `src/scan.rs`. Committed f239d532.
- Translated `flush_str` (15948) to `src/string_pool.rs`. Committed a9eac0dc.
- Translated `pseudo_close` (2766) to `src/input.rs`. Added `pseudo_files` global. Committed a58a8268.
- Translated `get_input_normalization_state` (9073), `get_tracing_fonts_state` (9082), `scan_pdf_ext_toks` (15945) to `src/scan.rs`. Added `eqtb_is_null()` to globals.rs. Committed 2f8134e1.
- Updated MODULE_STATUS.md, SESSION_LOG.md, PROGRESS.md.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Trip: 2/2
- Streak: 23
- 193 Rust functions total; 11 new this session.
- xetex-xetex0.c: 16157 LOC, ~166 C functions remaining.

**Surprises / gotchas:**
- `fract`'s multi-goto structure: `goto found`/`goto found1`/`goto too_big`/`goto done` all go forward, so labeled blocks in Rust (`if n != 0 { ... }`) + early returns for `too_big` gives a clean translation.
- `scan_pdf_ext_toks` and `scan_normal_glue`/`scan_mu_glue` are trivial one-liner wrappers — easy wins.
- TRIP test is in the `tectonic` package, not `tectonic_engine_xetex`.

**For the next session:**
- `begin_file_reading`/`end_file_reading` (~3113): needs `max_in_open`, `first`, `buf_size`, `source_filename_stack`, `eof_seen`, `MID_LINE`, `input_file`.
- `scan_expr` (~6379): complex recursive scanner; depends on `get_x_token`, `back_input` (done), etc.
- `pseudo_start` (~6614): uses `scan_general_text`, `token_show`, `flush_list`, `make_string`.
- Small helper cluster: `bad_utf8_warning` (~9393), `do_locale_linebreaks` (~9338) — complex.

---

## Session 22 — 2026-04-19 — input-stack, SA alloc, math node alloc (11 functions)

**Goal:** Translate end_token_list cluster, SA node allocators, math node constructors, and misc helpers.

**Starting state:**
- Parity: 29/29
- Streak: 21

**Work done:**
- Added globals: `max_in_stack`, `stack_size`, `param_ptr`/`param_stack`/`max_param_stack`, `cur_ptr`, `sa_null`.
- Added `cur_input` field setters: `state`, `index`, `set_cur_input` (full replace).
- Created `src/input.rs`: `end_token_list`, `back_input`, `back_error`, `ins_error` (lines 3108–3179). Committed d55b8f05.
- Added `new_index`, `find_sa_element` to `src/save.rs` (lines 4111–4264). Added `sa_alloc_chain` helper for the multi-goto allocation chain. Committed 250f183b.
- Added `new_noad`, `new_style`, `new_choice` to `src/mem.rs` (lines 10542–10576). Committed 2334ba03.
- Added `max_hyphenatable_length` to `src/scan.rs` (line 11330). Committed 4394fae3.
- Added `new_margin_kern` to `src/mem.rs` (line 9838). Committed 5147b292.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 22
- 182 Rust functions total; 11 new this session.
- xetex-xetex0.c: 16363 LOC, ~177 C functions remaining.

**Surprises / gotchas:**
- `find_sa_element`'s multi-goto chain (not_found, lab46–49) is cleanest in Rust as a single helper `sa_alloc_chain(q, levels)` that processes all remaining levels from a given entry point; avoids repetition while correctly mirroring C fall-through semantics.
- `new_margin_kern` in C: the `p` parameter is unused — confirmed by reading the body.
- `back_input` removes the C `extern "C" { fn back_input(); }` from `save.rs` since it's now a Rust function called via `crate::input::back_input()`.

**For the next session:**
- `begin_file_reading`/`end_file_reading` (~3113): need `in_open`, `max_in_open`, `first`, `buf_size`, `source_filename_stack`, `full_source_filename_stack`, `eof_seen`, `line_stack`, `pseudo_close`, `u_close`, `input_file` globals.
- `scan_math` (~4554): depends on `get_x_token`, `eqtb`, math globals — complex, approach carefully.
- `new_noad` ✓, `new_style` ✓, `new_choice` ✓ done.
- `show_save_groups` (~11356): complex multi-goto; approach with label-loops carefully.

---

## Session 21 — 2026-04-19 — SA cluster + unsave + show_cur_cmd_chr (11 functions)

**Goal:** Translate sparse-array register save/restore cluster, unsave, and show_cur_cmd_chr.

**Starting state:**
- Parity: 29/29
- Streak: 20

**Work done:**
- Added `sa_root`, `sa_chain`, `sa_level` globals to `globals.rs`.
- Added `cur_tok`, `align_state`, `shown_mode`, `cur_list_mode()` globals/accessors.
- Added `set_cur_input_loc`, `set_cur_input_start`, `save_stack_word` reader.
- Translated SA cluster to `src/save.rs`: `delete_sa_ref`, `sa_save`, `sa_destroy`, `sa_def`, `sa_w_def`, `gsa_def`, `gsa_w_def`, `sa_restore` (lines 2789–2974). Committed 2c28bd03.
- Translated `unsave` (line 2814) to `src/save.rs`. Complex save-stack restoration: handles INSERT_TOKEN (align_state), RESTORE_SA, RESTORE_OLD_VALUE/ZERO, LEVEL_BOUNDARY. Committed ddf8d1cc.
- Translated `show_cur_cmd_chr` (line 2824) to `src/output.rs`. Committed 1a27b4e9.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 21
- 171 Rust functions total; 11 new this session.
- xetex-xetex0.c: 16629 LOC, ~188 C functions remaining.

**Surprises / gotchas:**
- `delete_sa_ref` uses `TEX_NULL` as reference-count "zero" sentinel (not 0), so the ref-count starts at TEX_NULL and gets incremented. Translating arithmetically gives the right behavior.
- `sa_save` modifies `i = TOK_VAL_LIMIT` only in one branch of the DIMEN_VAL_LIMIT check, requiring `let mut i`.
- `unsave` goto→loop: `goto done` just breaks out of the inner loop; clean `break` in Rust.
- `shown_mode` is `i16` in C (short), but `cur_list.mode` is also `i16` — no cast needed.

**For the next session:**
- `end_token_list` (~line 3412): needs `param_ptr`, `param_stack` globals.
- `back_input` (~line 3440): uses `cur_tok` (now done), `V_TEMPLATE` constant, `get_avail`.
- `show_save_groups` (~line 12120): complex multi-goto — use labeled blocks in Rust.
- `scan_math` (~line 5108): math scanning; depends on several math-mode globals.

---

## Session 20 — 2026-04-19 — Phase 12+: id_lookup, prim_lookup, mu_error + 15 scan helpers (16 functions)

**Goal:** Translate hash-table lookup functions and scanner range-validation helpers.

**Starting state:**
- Parity: 29/29
- Streak: 19

**Work done:**
- Added `hash_used`, `hash_extra`, `hash_high`, `no_new_control_sequence`, `prim_used` globals; `hash_s0/s1`, `prim_s0/s1` setter accessors; `set_cur_val`; `max_reg_num`/`max_reg_help_line` to `globals.rs`.
- Created `src/hash.rs`: translated `id_lookup` (line 2713) and `prim_lookup` (line 2803). Committed 8ae4e8d1.
- Created `src/scan.rs`: translated `mu_error`, `scan_char_class`, `scan_char_class_not_ignored`, `scan_eight_bit_int`, `scan_usv_num`, `scan_char_num`, `scan_math_class_int`, `scan_math_fam_int`, `scan_four_bit_int`, `scan_fifteen_bit_int`, `scan_delimiter_int` (lines 5050–5419). Committed 467271cd.
- Added `scan_register_num`, `scan_four_bit_int_or_18` to `src/scan.rs` (lines 5245, 5263). Committed c54d3a06.
- Added `scan_xetex_math_char_int` to `src/scan.rs` (line 5168). Committed 64deab5b.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 20
- 160 Rust functions total; 16 new this session.
- xetex-xetex0.c: 16933 LOC, ~198 C functions remaining.

**Surprises / gotchas:**
- `prim_lookup` uses a local `l=0` when `s <= BIGGEST_CHAR`; Rust `length(s)` returns 1 for single-char strings — must preserve the C `l` variable, not call `length(s)` in the comparison.
- `cur_length()` is an inline C function (not an extern), so it was re-implemented as a private Rust fn in `hash.rs`.
- `scan_xetex_math_char_int`: `math_char(x)` is a C macro `((unsigned)(x) & 0x1FFFFF)` — inlined as `(x as u32) & 0x1F_FFFF` in Rust.

**For the next session:**
- `unsave` (~line 2992): complex goto→loop; approach carefully.
- `show_cur_cmd_chr` (~line 3073): diagnostic tracing; needs `print_cmd_chr` via extern "C".
- `scan_math` (~line 5108) and `set_math_char` (~line 5299): next in scanner cluster.
- `show_save_groups` (~line 12120): complex multi-goto, defer.
- Consider `delete_sa_ref` / `sa_save` / `sa_destroy` / `sa_def` cluster (lines 2789+).

---

## Session 19 — 2026-04-18 — Phase 11+: print_font_and_char, print_mark, print_meaning, group/if/file_warning (6 functions)

**Goal:** Translate the warm-up display helpers and file-nesting warning functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Streak: 18

**Work done:**
- Added new globals to `globals.rs`: `font_max`, `cur_cmd` (u8), `cur_mark[5]` (i32),
  `input_stack` (*mut InputState), `input_ptr`, `base_ptr`, `grp_stack` (*mut i32),
  `if_stack` (*mut i32), `cond_ptr`, `if_limit` (u8), `cur_if` (i16), `if_line`.
  Added `InputState: Copy + Clone`. Added `cur_input()` full-struct accessor.
- Added constants to `output.rs`: `FONT_ID_BASE`, `TOKEN_LIST`, `IF_TEST`, `TOP_BOT_MARK`,
  `CALL`, `FI_CODE`, `INT_PAR_TRACING_NESTING`. Added `print_cmd_chr` and `show_context`
  to `extern "C"` block.
- Translated `print_font_and_char` (line 225) → `src/output.rs`. (4a027338)
- Translated `print_mark` (line 240) → `src/output.rs`. (4a027338)
- Translated `print_meaning` (line 3383) → `src/output.rs`. Corrected `TOP_BOT_MARK`
  (was accidentally drafted as `IF_TEST` before review). (4a027338)
- Translated `group_warning` (line 2961) → `src/output.rs`. Uses `input_stack`/`grp_stack`
  globals; saves `cur_input` to `input_stack[base_ptr]` via new accessor. (4a027338)
- Translated `if_warning` (line 2998) → `src/output.rs`. Same pattern as group_warning
  but walks `if_stack`. (4a027338)
- Translated `file_warning` (line 3039) → `src/output.rs`. Saves/restores
  save-stack + cond_ptr chain to emit per-file unclosed group/if warnings. (4a027338)
- Deleted all 6 C bodies, replaced with `/* ... translated to Rust (src/output.rs). */` stubs.
- Updated `MODULE_STATUS.md`, `SESSION_LOG.md`, `PROGRESS.md`.

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 19
- 144 Rust functions total; 6 new this session.
- xetex-xetex0.c: 17329 LOC, 214 C functions remaining.

**Surprises / gotchas:**
- `print_meaning` initially had `IF_TEST` where the C uses `TOP_BOT_MARK` — caught during delete-phase review.
- `&raw::cur_input` cannot be accessed from outside `mod raw`; replaced with new `cur_input()` accessor.
- `InputState` needed `Copy + Clone` derives since we return it by value from `cur_input()`.
- `set_cur_group(save_stack_b16s0(...))` required `as u8` cast (u16 → u8).

**For the next session:**
- `id_lookup` (original line 2734, current ~2680): next pure hash-lookup candidate.
- `prim_lookup` (original line 2824): same.
- `unsave` (original ~3953): complex goto→loop; approach carefully.
- `eq_define` / `geq_define` already done. Next save-stack function is `unsave`.
- Consider `show_cur_cmd_chr` (diagnostic tracing) once `print_cmd_chr` is translated.
- Remaining Phase 11 candidates: `print_if_line` (3475), `show_save_groups` (3489).

---

## Session 18 — 2026-04-18 — Phase 12: 22 more functions (print_*, eq_*, save-stack, font errors)

**Goal:** continue translating scattered functions from xetex-xetex0.c.

**Starting state:**
- Parity: 29/29
- Streak: 17

**Work done:**
- `prepare_mag` (3997) → `src/save.rs`. Added mag_set global, variadic ttstub_diag_printf extern. (46bcab4a)
- `push_nest`, `pop_nest` (1323, 1339) → `src/mem.rs`. Added ListStateRecord #[repr(C)] to mem_types.rs. Added nest, nest_ptr, max_nest_stack, nest_size, cur_list globals. (8d9c24a7)
- `eq_destroy`, `eq_define`, `geq_define` (3831, 3867, 3886) → `src/save.rs`. (637a3fb7)
- `prev_rightmost` (140) → `src/mem.rs`; `get_microinterval` (155) → `src/texmfmp.rs`. Added epochseconds, microseconds globals. (bb6c2214)
- `token_show` (3893) → `src/output.rs`. (f1fcfc1a)
- `print_skip_param` (379), `print_param` (1423), `print_length_param` (1676) → `src/output.rs`. Large dispatch tables using numeric INT_PAR__*/GLUE_PAR__*/DIMEN_PAR__* values. (d3fd853c)
- `print_style` (319), `print_fam_and_char` (294), `print_delimiter` (307) → `src/output.rs`. Added mem_b16s2/b16s3 accessors. (45b16fdd)
- `print_rule_dimen` (250), `print_glue` (258), `print_spec` (274) → `src/output.rs`. Added NULL_FLAG/NORMAL/FIL/FILLL constants; lo_mem_max import. (cb4ec99f)
- `not_aat_font_error`, `not_aat_gr_font_error`, `not_ot_font_error`, `not_native_font_error` (2727-2763) → `src/errors.rs`. Added font_name global. Factored to private helper. (e4621a6b)
- Updated plan docs (this commit).

**Ending state:**
- Parity: 29/29 (delta: 0)
- Streak: 18
- 138 Rust functions total; 22 new this session.

**Surprises / gotchas:**
- `print_delimiter` uses i64 arithmetic to match C's implicit `long` promotion (a * 4096 overflows i32).
- `print_param` INT_PAR__xetex_linebreak_locale (66) is NOT in the C switch — left out of Rust match.
- `ListStateRecord` needs `#[repr(C)]` with 2-byte implicit padding after `mode: i16`, verified by size_of assert == 32.
- `not_font_error_*` all have identical structure; factored to `not_font_error_common` private helper.

**For the next session:**
- `print_mark` (240): calls show_token_list (C), hi_mem_min, mem_end, max_print_line — all available.
- `print_font_and_char` (225): calls print_esc, hash accessor, mem_end — moderate.
- `print_meaning` (3900): calls print_cmd_chr (C), cur_cmd global, cur_mark global.
- `id_lookup` (2779), `prim_lookup` (2869): hash/primitive table lookups.
- Consider `unsave` (3953 in current file): complex goto→loop, needs careful translation.
- `group_warning`, `if_warning`, `file_warning`: need grp_stack, if_stack, input_stack globals.

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
