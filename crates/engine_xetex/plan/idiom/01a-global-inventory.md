# 01a — Global inventory

Generated 2026-05-23 from `src/globals.rs`, promoted `pub static mut` definitions in `src/*.rs`, non-macOS C files compiled by `build.rs`, and `src/ini.rs` `#[link_name]` aliases. macOS-only references are intentionally ignored per Stage 2 scope.

## Summary

- Raw extern globals in `globals.rs`: 384
- Promoted Rust `pub static mut` globals outside `globals.rs`: 16
- Main inventory total: 400
- Additional private module-local `static mut` values, listed separately: 18
- Main-inventory cluster counts:
  - `config`: 7
  - `eqtb`: 11
  - `errors`: 27
  - `fonts`: 42
  - `hyph`: 49
  - `input`: 42
  - `linebreak`: 34 (was `misc`; resolved session 44 — see decision block)
  - `mem`: 9
  - `save_stack`: 8
  - `scanner`: 100
  - `shipout`: 58
  - `strings`: 13
- Private-static cluster counts:
  - `fonts`: 5
  - `input`: 5
  - `shipout`: 4 (was `misc`; pagebuilder cost scratch, folded — session 44)
  - `synctex`: 1 (was `misc`; resolved session 44)
  - `texmf`: 3 (was `misc`; resolved session 44)

## Classification notes

- `C/link status` lists non-macOS C definitions, non-macOS C `extern` declarations/references, and Rust `#[link_name]` aliases only.
- `link src/ini.rs:<line>` means init/load/cleanup glue reaches the symbol by C name; the Phase-B commit that moves that symbol must remove or redirect that alias/free path.
- The `misc` holding bucket is **resolved as of session 44**; see the
  "Sub-struct decisions" block below. No row is tagged `misc` anymore.
- Accessor names are mechanically detected by exact `raw::<symbol>` references in `globals.rs`; module-local direct extern aliases outside `globals.rs` are reflected in `C/link status`, not in `accessors`.
- `xetex-macos.c` is intentionally excluded and must not block Linux-validated idiomatization.

## Sub-struct decisions (misc resolution — session 44)

The former `misc` bucket (34 raw extern globals + 8 private module-local
statics) was resolved into concrete sub-structs this session. **Planning
only — no Rust/C code changed.** Verification done before deciding:

- All 34 raw externs are line-breaking / LR-reverse / hyphenation-attempt
  scratch, used almost entirely in `src/linebreak.rs`
  (`last_leftmost_char`/`last_rightmost_char` are also read by the
  LR-reverse helper in `src/scan.rs`). They are line-breaker working state
  (`line_break`/`try_break`/`post_line_break`/`hyphenate`), run **per
  paragraph**, not on the tokenizer hot path — so they sit in the "cold
  misc scratch" bucket and are safe to bridge through the thread-local in
  Phase B (the only hard deferral is the tokenizer scalars).
- None of the 34 have a `#[link_name]` alias in `src/ini.rs` and none are
  in the format dump (grep-confirmed): they are pure BSS scalars/inline
  arrays. So the migration is a **scalar/inline-array move** (sessions
  39–43 shape), *not* an ownership slice — no `ini.rs` alloc, no
  `tt_cleanup` free, no format-dump reconciliation.
- The fields STATUS's earlier prose floated as `MathState` / `DviState` /
  `OutputState` / `NativeTextState` candidates (`cur_mlist`, `cur_style`,
  `cur_size`, `cur_mu`, `mlist_penalties`, `cur_f/c/i`, `total_pages`,
  `max_v/h`, `cur_h/v`, `cur_box`, `after_token`, `cur_order`,
  `native_text`, `space_class`, `LR_ptr`, `cur_dir`, …) are **already
  classified into `scanner`/`shipout`** in this inventory — classified,
  not yet migrated. They need **no new sub-struct**; they migrate into
  their existing assigned clusters. No `MathState`/`DviState` is created.

| Decision | Members | Migration shape | Target session |
|---|---|---|---|
| **`LinebreakState`** (new sub-struct on `EngineState`) | the 34 former-`misc` raw externs: `last_leftmost_char`, `last_rightmost_char`, `hlist_stack[513]`, `hlist_stack_level`, `first_p`, `global_prev_p`, `cur_p`, `last_special_line`, `first_width`, `second_width`, `first_indent`, `second_indent`, `best_bet`, `best_line`, `hn`, `ha`, `hb`, `passive`, `cur_active_width[7]`, `background[7]`, `break_width[7]`, `best_place[4]`, `best_pl_line[4]`, `disc_width`, `final_pass`, `threshold`, `minimal_demerits[4]`, `minimum_demerits`, `easy_line`, `do_last_line_fit`, `active_node_size`, `fill_width[3]`, `best_pl_short[4]`, `best_pl_glue[4]` | scalar/inline-array move. **`hlist_stack` (`[i32;513]`, ~2 KB) heap-back as `Vec<i32>` sized 513** — `EngineState` lives *inline* in the thread-local `RefCell` (not boxed), so a 2 KB inline array risks the session-26 `engine_rust` thread-stack overflow during `EngineState::default()`. The small arrays (`[i32;7]`, `[i32;4]`, `[i32;3]`) stay inline. | 45 |
| **`ShipoutState`** (extend existing) | the 4 pagebuilder private statics in `src/pagebuilder.rs`: `BEST_PAGE_BREAK`, `BEST_SIZE`, `LEAST_PAGE_COST`, `PAGE_MAX_DEPTH` | private Rust statics (no C def, no alias). Fold beside the page-builder fields already in `ShipoutState` (`page_contents`, `page_so_far`, `page_tail`, `last_glue/penalty/kern`, `insert_penalties`, `output_active`, `dead_cycles`); repoint the in-module accessors. | 46 |
| **`EngineState.synctex: SynctexCtxt`** (new field) | `SYNCTEX_CTXT` (`src/synctex.rs`) | move the whole context struct in as one field. `SynctexCtxt` holds file-handle/counter state — confirm a `Default` impl (or write one) and route any open/close lifecycle through the existing synctex entry points. | 46 |
| **`TexmfState`** (new sub-struct) | `LAST_SOURCE_NAME` (`*mut c_char`), `LAST_LINENO` (`i32`), `START_TIME_STR[30]` (`src/texmfmp.rs`) | small io/job-context struct. **`LAST_SOURCE_NAME` is an owned C string** (`strdup`/`free` lifecycle) — keep the raw pointer in Phase B and record an ownership note (eventual `Option<CString>`); do not double-free. | 46 |

**Newtype-debt note (added at migration time per §4, not now):** many of
the linebreak fields are semantic typedefs and will need worklist rows in
the migrating commit — the width/demerit arrays and scalars are `scaled_t`
(`cur_active_width`, `background`, `break_width`, `disc_width`,
`first/second_width`, `first/second_indent`, `fill_width`,
`best_height_plus_depth`); the demerit fields are `int32_t` but
`minimal_demerits`/`minimum_demerits`/`best_pl_glue` mix scaled and plain;
the pointer-ish fields (`cur_p`, `first_p`, `global_prev_p`, `passive`,
`best_bet`, `best_place`, `ha`, `hb`) are mem halfwords (`MemPtr`); `hn`
is `small_number`; `active_node_size` is `small_number`; the `*_line`
fields are line numbers. Plain counters (`threshold`, `easy_line`) need
none. Resolve per field with `xetex-xetexd.h` at session 45.

## Raw extern globals

| name | type | cluster | defined where | accessors | C/link status | notes |
|---|---|---|---|---|---|---|
| `arith_error` | `bool` | `errors` | `src/globals.rs:57` C extern | `arith_error`, `set_arith_error` | def xetex/xetex-ini.c |  |
| `tex_remainder` | `i32` | `errors` | `src/globals.rs:59` C extern | `tex_remainder`, `set_tex_remainder` | def xetex/xetex-ini.c |  |
| `dig` | `[u8; 23]` | `errors` | `src/globals.rs:61` C extern | `dig_entry`, `set_dig_entry` | def xetex/xetex-ini.c |  |
| `randoms` | `[i32; 55]` | `errors` | `src/globals.rs:65` C extern | `random_entry`, `set_random_entry` | def xetex/xetex-ini.c |  |
| `j_random` | `u8` | `errors` | `src/globals.rs:67` C extern | `j_random`, `set_j_random` | def xetex/xetex-ini.c |  |
| `cur_input` | `super::InputState` | `input` | `src/globals.rs:70` C extern | `cur_input_synctex_tag`, `cur_input`, `set_cur_input_loc`, `set_cur_input_start`, `set_cur_input_state`, `set_cur_input_index`, ... (+4) | def xetex/xetex-ini.c |  |
| `open_parens` | `i32` | `input` | `src/globals.rs:72` C extern | `open_parens`, `set_open_parens` | def xetex/xetex-ini.c |  |
| `line` | `i32` | `input` | `src/globals.rs:74` C extern | `line`, `set_line` | def xetex/xetex-ini.c; ref xetex/xetex-linebreak.c,xetex/xetex-texmfmp.c |  |
| `eqtb` | `*mut crate::mem_types::MemoryWord` | `eqtb` | `src/globals.rs:78` C extern | `eqtb_is_null`, `eqtb_word`, `set_eqtb_word`, `eqtb_b32s1`, `set_eqtb_b32s1`, `eqtb_b16s1`, ... (+3) | def xetex/xetex-ini.c; link src/ini.rs:1864,src/ini.rs:3598 |  |
| `str_start` | `*mut i32` | `strings` | `src/globals.rs:82` C extern | `str_start`, `set_str_start` | def xetex/xetex-ini.c; link src/ini.rs:1866,src/ini.rs:3602 |  |
| `str_ptr` | `i32` | `strings` | `src/globals.rs:84` C extern | `str_ptr`, `set_str_ptr` | def xetex/xetex-ini.c |  |
| `temp_ptr` | `i32` | `mem` | `src/globals.rs:86` C extern | `temp_ptr`, `set_temp_ptr` | def xetex/xetex-ini.c; ref xetex/xetex-shipout.c |  |
| `mem` | `*mut MemoryWord` | `mem` | `src/globals.rs:92` C extern | `mem_b32s0`, `set_mem_b32s0`, `mem_b32s1`, `set_mem_b32s1`, `mem_b16s0`, `set_mem_b16s0`, ... (+15) | def xetex/xetex-ini.c; link src/ini.rs:1856,src/ini.rs:3600 |  |
| `lo_mem_max` | `i32` | `mem` | `src/globals.rs:93` C extern | `lo_mem_max`, `set_lo_mem_max` | def xetex/xetex-ini.c |  |
| `hi_mem_min` | `i32` | `mem` | `src/globals.rs:94` C extern | `hi_mem_min`, `set_hi_mem_min` | def xetex/xetex-ini.c |  |
| `avail` | `i32` | `mem` | `src/globals.rs:95` C extern | `avail`, `set_avail` | def xetex/xetex-ini.c |  |
| `mem_end` | `i32` | `mem` | `src/globals.rs:96` C extern | `mem_end`, `set_mem_end` | def xetex/xetex-ini.c |  |
| `rover` | `i32` | `mem` | `src/globals.rs:97` C extern | `rover`, `set_rover` | def xetex/xetex-ini.c |  |
| `selector` | `i32` | `shipout` | `src/globals.rs:101` C extern | `selector`, `set_selector` | def xetex/xetex-ini.c |  |
| `doing_special` | `bool` | `shipout` | `src/globals.rs:103` C extern | `doing_special`, `set_doing_special` | def xetex/xetex-ini.c |  |
| `term_offset` | `i32` | `shipout` | `src/globals.rs:105` C extern | `term_offset`, `set_term_offset` | def xetex/xetex-ini.c |  |
| `file_offset` | `i32` | `shipout` | `src/globals.rs:106` C extern | `file_offset`, `set_file_offset` | def xetex/xetex-ini.c |  |
| `rust_stdout` | `usize` | `shipout` | `EngineState.shipout.rust_stdout` (Rust-owned) | `rust_stdout`, `set_rust_stdout` | MOVED session 23: C def + alias removed |
| `log_file` | `usize` | `shipout` | `EngineState.shipout.log_file` (Rust-owned) | `log_file`, `set_log_file` | MOVED session 23: C def removed |
| `write_file` | `[usize; 16]` | `shipout` | `EngineState.shipout.write_file` (Rust-owned) | `write_file`, `set_write_file` | MOVED session 19: C def removed |
| `write_open` | `[bool; 18]` | `shipout` | `EngineState.shipout.write_open` (Rust-owned) | `write_open`, `set_write_open` | MOVED session 19: C def removed |
| `current_diagnostic` | `*mut std::ffi::c_void` | `shipout` | `EngineState.shipout.current_diagnostic` (Rust-owned) | `set_current_diagnostic`, `current_diagnostic` | MOVED session 23: C def removed |
| `pseudo_files` | `i32` | `input` | `src/globals.rs:121` C extern | `pseudo_files`, `set_pseudo_files` | def xetex/xetex-ini.c |  |
| `in_open` | `i32` | `input` | `src/globals.rs:123` C extern | `in_open`, `set_in_open` | def xetex/xetex-ini.c |  |
| `full_source_filename_stack` | `*mut i32` | `input` | `src/globals.rs:125` C extern | `full_source_filename_stack`, `set_full_source_filename_stack` | def xetex/xetex-ini.c; link src/ini.rs:1846,src/ini.rs:3584 |  |
| `line_stack` | `*mut i32` | `input` | `src/globals.rs:127` C extern | `line_stack`, `set_line_stack` | def xetex/xetex-ini.c; link src/ini.rs:1836,src/ini.rs:3574 |  |
| `file_line_error_style_p` | `i32` | `config` | `src/globals.rs:129` C extern | `file_line_error_style_p` | def xetex/xetex-ini.c; link src/ini.rs:1784 |  |
| `hash` | `Vec<crate::mem_types::B32x2>` | `eqtb` | `EngineState.eqtb.hash` (Rust-owned) | `hash_s0`, `set_hash_s0`, `hash_s1`, `set_hash_s1` | MOVED session 52: C def, static yhash, and FFI yhash helpers removed |
| `hash_used` | `i32` | `eqtb` | `src/globals.rs:134` C extern | `hash_used`, `set_hash_used` | def xetex/xetex-ini.c |  |
| `hash_extra` | `i32` | `eqtb` | `EngineState.eqtb.hash_extra` (Rust-owned) | `hash_extra`, `set_hash_extra` | MOVED session 52: C def and link alias removed |
| `hash_high` | `i32` | `eqtb` | `src/globals.rs:136` C extern | `hash_high`, `set_hash_high` | def xetex/xetex-ini.c |  |
| `no_new_control_sequence` | `bool` | `eqtb` | `src/globals.rs:137` C extern | `no_new_control_sequence`, `set_no_new_control_sequence` | def xetex/xetex-ini.c |  |
| `prim` | `[crate::mem_types::B32x2; 2101]` | `eqtb` | EngineState.eqtb.prim (Rust-owned Vec) | `prim_s0`, `set_prim_s0`, `prim_s1`, `set_prim_s1` | MOVED session 49: C def removed; inline BSS, sized in EqtbState::default; format dump/undump via prim_s0/s1 accessors (transparent) |
| `prim_used` | `i32` | `eqtb` | `src/globals.rs:141` C extern | `prim_used`, `set_prim_used` | def xetex/xetex-ini.c |  |
| `eqtb_top` | `i32` | `eqtb` | `EngineState.eqtb.eqtb_top` (Rust-owned) | `eqtb_top`, `set_eqtb_top` | MOVED session 52: C def and link alias removed |
| `max_strings` | `i32` | `strings` | `src/globals.rs:146` C extern | `max_strings` | def xetex/xetex-ini.c; link src/ini.rs:1792 |  |
| `init_str_ptr` | `i32` | `strings` | `src/globals.rs:147` C extern | `init_str_ptr` | def xetex/xetex-ini.c; link src/ini.rs:1874 |  |
| `init_pool_ptr` | `i32` | `strings` | `src/globals.rs:148` C extern | `init_pool_ptr` | def xetex/xetex-ini.c; link src/ini.rs:1876 |  |
| `font_mem_size` | `i32` | `fonts` | `EngineState.fonts.font_mem_size` (Rust-owned) | `font_mem_size`, `set_font_mem_size` | MOVED session 30: C def + alias removed |
| `buffer` | `Vec<i32>` | `input` | `EngineState.input.buffer` (Rust-owned) | `buffer_entry`, `set_buffer_entry`, `buffer_ptr` | MOVED session 15: C def + both aliases removed |
| `help_line` | `[*const libc::c_char; 6]` | `errors` | `src/globals.rs:156` C extern | `help_line`, `set_help_line` | def xetex/xetex-ini.c |  |
| `help_ptr` | `u8` | `errors` | `src/globals.rs:157` C extern | `help_ptr`, `set_help_ptr` | def xetex/xetex-ini.c |  |
| `two_to_the` | `[i32; 31]` | `errors` | `src/globals.rs:160` C extern | `two_to_the`, `set_two_to_the` | def xetex/xetex-ini.c |  |
| `spec_log` | `[i32; 29]` | `errors` | `src/globals.rs:161` C extern | `spec_log`, `set_spec_log` | def xetex/xetex-ini.c |  |
| `interaction` | `u8` | `errors` | `src/globals.rs:165` C extern | `interaction`, `set_interaction` | def xetex/xetex-ini.c |  |
| `history` | `i32` | `errors` | `src/globals.rs:167` C extern | `history`, `set_history` | def xetex/xetex-ini.c |  |
| `error_count` | `i8` | `errors` | `src/globals.rs:168` C extern | `error_count`, `set_error_count` | def xetex/xetex-ini.c |  |
| `use_err_help` | `bool` | `errors` | `src/globals.rs:169` C extern | `use_err_help`, `set_use_err_help` | def xetex/xetex-ini.c |  |
| `long_help_seen` | `bool` | `errors` | `src/globals.rs:171` C extern | `long_help_seen`, `set_long_help_seen` | def xetex/xetex-ini.c |  |
| `halt_on_error_p` | `i32` | `config` | `src/globals.rs:172` C extern | `halt_on_error_p`, `set_halt_on_error_p` | def xetex/xetex-ini.c |  |
| `log_opened` | `bool` | `errors` | `src/globals.rs:174` C extern | `log_opened`, `set_log_opened` | def xetex/xetex-ini.c |  |
| `job_name` | `i32` | `strings` | `src/globals.rs:175` C extern | `job_name`, `set_job_name` | def xetex/xetex-ini.c |  |
| `texmf_log_name` | `i32` | `strings` | `src/globals.rs:176` C extern | `texmf_log_name`, `set_texmf_log_name` | def xetex/xetex-ini.c |  |
| `in_initex_mode` | `bool` | `config` | `src/globals.rs:179` C extern | `in_initex_mode`, `set_in_initex_mode` | def xetex/xetex-ini.c |  |
| `synctex_enabled` | `i32` | `config` | `src/globals.rs:180` C extern | `set_synctex_enabled` | def xetex/xetex-ini.c |  |
| `semantic_pagination_enabled` | `bool` | `config` | `src/globals.rs:181` C extern | `semantic_pagination_enabled`, `set_semantic_pagination_enabled` | def xetex/xetex-ini.c |  |
| `shell_escape_enabled` | `bool` | `config` | `src/globals.rs:182` C extern | `shell_escape_enabled`, `set_shell_escape_enabled` | def xetex/xetex-ini.c |  |
| `max_print_line` | `i32` | `config` | `src/globals.rs:185` C extern | `max_print_line` | def xetex/xetex-ini.c; link src/ini.rs:1820 |  |
| `tally` | `i32` | `errors` | `src/globals.rs:187` C extern | `tally`, `set_tally` | def xetex/xetex-ini.c |  |
| `error_line` | `i32` | `errors` | `src/globals.rs:189` C extern | `error_line` | def xetex/xetex-ini.c; link src/ini.rs:1816 |  |
| `half_error_line` | `i32` | `errors` | `src/globals.rs:190` C extern | `half_error_line` | def xetex/xetex-ini.c; link src/ini.rs:1818 |  |
| `trick_buf` | `[u16; 256]` | `errors` | `src/globals.rs:191` C extern | `trick_buf`, `set_trick_buf` | def xetex/xetex-ini.c |  |
| `trick_count` | `i32` | `errors` | `src/globals.rs:192` C extern | `set_trick_count`, `trick_count` | def xetex/xetex-ini.c |  |
| `first_count` | `i32` | `errors` | `src/globals.rs:193` C extern | `first_count`, `set_first_count` | def xetex/xetex-ini.c |  |
| `string_vacancies` | `i32` | `strings` | `src/globals.rs:196` C extern | `string_vacancies` | def xetex/xetex-ini.c; link src/ini.rs:1788 |  |
| `pool_size` | `i32` | `strings` | `src/globals.rs:197` C extern | `pool_size` | def xetex/xetex-ini.c; link src/ini.rs:1786 |  |
| `str_pool` | `*mut u16` | `strings` | `src/globals.rs:198` C extern | `str_pool_entry`, `str_pool_ptr`, `set_str_pool` | def xetex/xetex-ini.c; link src/ini.rs:1868,src/ini.rs:3604 |  |
| `pool_ptr` | `i32` | `strings` | `src/globals.rs:199` C extern | `pool_ptr`, `set_pool_ptr` | def xetex/xetex-ini.c |  |
| `name_of_file` | `*mut libc::c_char` | `input` | `src/globals.rs:203` C extern | `name_of_file`, `set_name_of_file` | def xetex/xetex-ini.c |  |
| `name_length` | `i32` | `input` | `src/globals.rs:205` C extern | `set_name_length`, `name_length` | def xetex/xetex-ini.c |  |
| `name_of_file16` | `*mut u16` | `input` | `src/globals.rs:207` C extern | `name_of_file16`, `set_name_of_file16`, `name_of_file16_entry` | def xetex/xetex-ini.c |  |
| `name_length16` | `i32` | `input` | `src/globals.rs:209` C extern | `name_length16`, `set_name_length16` | def xetex/xetex-ini.c |  |
| `old_setting` | `u8` | `shipout` | `src/globals.rs:212` C extern | `old_setting`, `set_old_setting` | def xetex/xetex-ini.c |  |
| `epochseconds` | `i32` | `shipout` | `src/globals.rs:214` C extern | `epochseconds`, `set_epochseconds` | def xetex/xetex-ini.c |  |
| `microseconds` | `i32` | `shipout` | `src/globals.rs:215` C extern | `microseconds`, `set_microseconds` | def xetex/xetex-ini.c |  |
| `font_name` | `*mut i32` | `fonts` | `src/globals.rs:217` C extern | `font_name_entry`, `set_font_name_entry` | def xetex/xetex-ini.c; link src/ini.rs:1896,src/ini.rs:3624 |  |
| `font_info` | `*mut MemoryWord` | `fonts` | `src/globals.rs:219` C extern | `font_info_b32s1`, `set_font_info_b32s1`, `set_font_info_b16`, `font_info_b16` | def xetex/xetex-ini.c; link src/ini.rs:1870,src/ini.rs:3606 |  |
| `fmem_ptr` | `i32` | `fonts` | `EngineState.fonts.fmem_ptr` (Rust-owned) | `fmem_ptr`, `set_fmem_ptr` | MOVED session 39: C def removed |  |
| `font_ptr` | `i32` | `fonts` | `EngineState.fonts.font_ptr` (Rust-owned) | `font_ptr`, `set_font_ptr` | MOVED session 39: C def removed |  |
| `font_check` | `Vec<crate::mem_types::B16x4>` | `fonts` | `EngineState.fonts.font_check` (Rust-owned) | `set_font_check`, `font_check_entry` | MOVED session 32: C def + both aliases removed; format dump/undump through state |
| `xdv_buffer` | `*mut libc::c_char` | `fonts` | `src/globals.rs:223` C extern | `xdv_buffer_entry` | def xetex/xetex-ini.c |  |
| `font_params` | `*mut i32` | `fonts` | `src/globals.rs:224` C extern | `font_params`, `set_font_params` | def xetex/xetex-ini.c; link src/ini.rs:1894,src/ini.rs:3622 |  |
| `font_glue` | `*mut i32` | `fonts` | `src/globals.rs:225` C extern | `font_glue`, `set_font_glue` | def xetex/xetex-ini.c; link src/ini.rs:1904,src/ini.rs:3632 |  |
| `param_base` | `*mut i32` | `fonts` | `EngineState.fonts.param_base` (Rust-owned) | `param_base`, `set_param_base` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `cur_cs` | `i32` | `scanner` | `src/globals.rs:228` C extern | `cur_cs`, `set_cur_cs` | def xetex/xetex-ini.c |  |
| `cur_tok` | `i32` | `scanner` | `src/globals.rs:230` C extern | `cur_tok`, `set_cur_tok` | def xetex/xetex-ini.c |  |
| `align_state` | `i32` | `scanner` | `src/globals.rs:232` C extern | `align_state`, `set_align_state` | def xetex/xetex-ini.c |  |
| `cur_chr` | `i32` | `scanner` | `src/globals.rs:234` C extern | `cur_chr`, `set_cur_chr` | def xetex/xetex-ini.c |  |
| `cur_cmd` | `u8` | `scanner` | `src/globals.rs:236` C extern | `cur_cmd`, `set_cur_cmd` | def xetex/xetex-ini.c |  |
| `cur_mark` | `[i32; 5]` | `scanner` | `src/globals.rs:238` C extern | `cur_mark`, `set_cur_mark` | def xetex/xetex-ini.c |  |
| `cur_val` | `i32` | `scanner` | `src/globals.rs:240` C extern | `cur_val`, `set_cur_val` | def xetex/xetex-ini.c |  |
| `cur_name` | `i32` | `scanner` | `src/globals.rs:242` C extern | `cur_name`, `set_cur_name` | def xetex/xetex-ini.c |  |
| `cur_area` | `i32` | `scanner` | `src/globals.rs:243` C extern | `cur_area`, `set_cur_area` | def xetex/xetex-ini.c |  |
| `cur_ext` | `i32` | `scanner` | `src/globals.rs:244` C extern | `cur_ext`, `set_cur_ext` | def xetex/xetex-ini.c |  |
| `name_in_progress` | `bool` | `scanner` | `src/globals.rs:246` C extern | `name_in_progress`, `set_name_in_progress` | def xetex/xetex-ini.c |  |
| `area_delimiter` | `i32` | `scanner` | `src/globals.rs:249` C extern | `area_delimiter`, `set_area_delimiter` | def xetex/xetex-ini.c |  |
| `ext_delimiter` | `i32` | `scanner` | `src/globals.rs:251` C extern | `ext_delimiter`, `set_ext_delimiter` | def xetex/xetex-ini.c |  |
| `quoted_filename` | `bool` | `scanner` | `src/globals.rs:253` C extern | `quoted_filename`, `set_quoted_filename` | def xetex/xetex-ini.c |  |
| `file_name_quote_char` | `u16` | `scanner` | `src/globals.rs:255` C extern | `file_name_quote_char`, `set_file_name_quote_char` | def xetex/xetex-ini.c |  |
| `stop_at_space` | `bool` | `scanner` | `src/globals.rs:257` C extern | `stop_at_space`, `set_stop_at_space` | def xetex/xetex-ini.c |  |
| `mag_set` | `i32` | `scanner` | `src/globals.rs:259` C extern | `mag_set`, `set_mag_set` | def xetex/xetex-ini.c |  |
| `font_max` | `i32` | `fonts` | `EngineState.fonts.font_max` (Rust-owned) | `font_max`, `set_font_max` | MOVED session 30: C def + alias removed |
| `input_stack` | `*mut super::InputState` | `input` | `src/globals.rs:265` C extern | `input_stack`, `set_input_stack` | def xetex/xetex-ini.c; link src/ini.rs:1832,src/ini.rs:3570 |  |
| `input_ptr` | `i32` | `input` | `src/globals.rs:266` C extern | `input_ptr`, `set_input_ptr` | def xetex/xetex-ini.c |  |
| `max_in_stack` | `i32` | `input` | `src/globals.rs:268` C extern | `max_in_stack`, `set_max_in_stack` | def xetex/xetex-ini.c |  |
| `stack_size` | `i32` | `input` | `src/globals.rs:270` C extern | `stack_size` | def xetex/xetex-ini.c; link src/ini.rs:1814 |  |
| `base_ptr` | `i32` | `input` | `src/globals.rs:272` C extern | `base_ptr`, `set_base_ptr` | def xetex/xetex-ini.c |  |
| `par_loc` | `i32` | `input` | `src/globals.rs:274` C extern | `par_loc`, `set_par_loc` | def xetex/xetex-ini.c |  |
| `par_token` | `i32` | `input` | `src/globals.rs:276` C extern | `par_token`, `set_par_token` | def xetex/xetex-ini.c |  |
| `param_stack` | `*mut i32` | `input` | `src/globals.rs:279` C extern | `param_stack`, `set_param_stack` | def xetex/xetex-ini.c; link src/ini.rs:1848,src/ini.rs:3586 |  |
| `param_ptr` | `i32` | `input` | `src/globals.rs:280` C extern | `param_ptr`, `set_param_ptr` | def xetex/xetex-ini.c |  |
| `max_param_stack` | `i32` | `input` | `src/globals.rs:281` C extern | `max_param_stack`, `set_max_param_stack` | def xetex/xetex-ini.c |  |
| `param_size` | `i32` | `input` | `src/globals.rs:283` C extern | `param_size` | def xetex/xetex-ini.c; link src/ini.rs:1810 |  |
| `cur_ptr` | `i32` | `scanner` | `src/globals.rs:287` C extern | `cur_ptr`, `set_cur_ptr` | def xetex/xetex-ini.c |  |
| `sa_null` | `MemoryWord` | `scanner` | `src/globals.rs:289` C extern | `sa_null`, `set_sa_null_b32s0`, `set_sa_null_b32s1` | def xetex/xetex-ini.c |  |
| `sa_root` | `[i32; 8]` | `scanner` | `src/globals.rs:293` C extern | `sa_root`, `set_sa_root` | def xetex/xetex-ini.c; link src/ini.rs:1954 |  |
| `sa_chain` | `i32` | `scanner` | `src/globals.rs:294` C extern | `sa_chain`, `set_sa_chain` | def xetex/xetex-ini.c |  |
| `sa_level` | `u16` | `scanner` | `src/globals.rs:295` C extern | `sa_level`, `set_sa_level` | def xetex/xetex-ini.c |  |
| `max_reg_num` | `i32` | `scanner` | `src/globals.rs:298` C extern | `max_reg_num`, `set_max_reg_num` | def xetex/xetex-ini.c |  |
| `max_reg_help_line` | `*const libc::c_char` | `scanner` | `src/globals.rs:299` C extern | `set_max_reg_help_line`, `max_reg_help_line` | def xetex/xetex-ini.c |  |
| `var_used` | `i32` | `mem` | `src/globals.rs:302` C extern | `var_used`, `set_var_used` | def xetex/xetex-ini.c |  |
| `dyn_used` | `i32` | `mem` | `src/globals.rs:303` C extern | `dyn_used`, `set_dyn_used` | def xetex/xetex-ini.c |  |
| `cs_count` | `i32` | `eqtb` | `src/globals.rs:305` C extern | `cs_count`, `set_cs_count` | def xetex/xetex-ini.c |  |
| `grp_stack` | `*mut i32` | `save_stack` | `src/globals.rs:309` C extern | `grp_stack`, `set_grp_stack` | def xetex/xetex-ini.c; link src/ini.rs:1840,src/ini.rs:3578 |  |
| `if_stack` | `*mut i32` | `input` | `src/globals.rs:312` C extern | `if_stack`, `set_if_stack` | def xetex/xetex-ini.c; link src/ini.rs:1842,src/ini.rs:3580 |  |
| `cond_ptr` | `i32` | `input` | `src/globals.rs:315` C extern | `cond_ptr`, `set_cond_ptr` | def xetex/xetex-ini.c |  |
| `if_limit` | `u8` | `input` | `src/globals.rs:317` C extern | `if_limit`, `set_if_limit` | def xetex/xetex-ini.c |  |
| `cur_if` | `i16` | `input` | `src/globals.rs:319` C extern | `cur_if`, `set_cur_if` | def xetex/xetex-ini.c |  |
| `if_line` | `i32` | `input` | `src/globals.rs:320` C extern | `if_line`, `set_if_line` | def xetex/xetex-ini.c |  |
| `skip_line` | `i32` | `input` | `src/globals.rs:322` C extern | `skip_line`, `set_skip_line` | def xetex/xetex-ini.c |  |
| `shown_mode` | `i16` | `scanner` | `src/globals.rs:325` C extern | `shown_mode`, `set_shown_mode` | def xetex/xetex-ini.c |  |
| `nest` | `*mut crate::mem_types::ListStateRecord` | `scanner` | `src/globals.rs:328` C extern | `restore_cur_list_from_nest`, `save_cur_list_to_nest`, `nest_mode`, `nest_tail`, `set_nest_tail`, `nest_e_tex_aux`, ... (+6) | def xetex/xetex-ini.c; link src/ini.rs:1828,src/ini.rs:3566 |  |
| `nest_ptr` | `i32` | `scanner` | `src/globals.rs:329` C extern | `nest_ptr`, `set_nest_ptr` | def xetex/xetex-ini.c |  |
| `max_nest_stack` | `i32` | `scanner` | `src/globals.rs:330` C extern | `max_nest_stack`, `set_max_nest_stack` | def xetex/xetex-ini.c |  |
| `nest_size` | `i32` | `scanner` | `src/globals.rs:331` C extern | `nest_size` | def xetex/xetex-ini.c; link src/ini.rs:1806 |  |
| `cur_list` | `crate::mem_types::ListStateRecord` | `scanner` | `src/globals.rs:333` C extern | `restore_cur_list_from_nest`, `save_cur_list_to_nest`, `cur_list_mode`, `set_cur_list_mode`, `cur_list_head`, `set_cur_list_head`, ... (+12) | def xetex/xetex-ini.c |  |
| `read_open` | `[u8; 17]` | `input` | `src/globals.rs:335` C extern | `read_open_entry`, `set_read_open_entry` | def xetex/xetex-ini.c |  |
| `read_file` | `[*mut libc::c_void; 16]` | `input` | `src/globals.rs:338` C extern | `read_file_entry`, `read_file_ptr` | def xetex/xetex-ini.c |  |
| `save_stack` | `*mut crate::mem_types::MemoryWord` | `save_stack` | `src/globals.rs:341` C extern | `save_stack_b32s1`, `save_stack_b32s0`, `set_save_stack_b32s1`, `set_save_stack_b32s0`, `save_stack_b16s1`, `save_stack_b16s0`, ... (+4) | def xetex/xetex-ini.c; link src/ini.rs:1830,src/ini.rs:3568 |  |
| `save_ptr` | `i32` | `save_stack` | `src/globals.rs:342` C extern | `save_ptr`, `set_save_ptr` | def xetex/xetex-ini.c |  |
| `max_save_stack` | `i32` | `save_stack` | `src/globals.rs:343` C extern | `max_save_stack`, `set_max_save_stack` | def xetex/xetex-ini.c |  |
| `save_size` | `i32` | `save_stack` | `src/globals.rs:344` C extern | `save_size` | def xetex/xetex-ini.c; link src/ini.rs:1812 |  |
| `cur_level` | `u16` | `save_stack` | `src/globals.rs:346` C extern | `cur_level`, `set_cur_level` | def xetex/xetex-ini.c |  |
| `cur_group` | `u8` | `save_stack` | `src/globals.rs:348` C extern | `cur_group`, `set_cur_group` | def xetex/xetex-ini.c |  |
| `cur_boundary` | `i32` | `save_stack` | `src/globals.rs:350` C extern | `cur_boundary`, `set_cur_boundary` | def xetex/xetex-ini.c |  |
| `_xeq_level_array` | `[u16; 1_114_730]` | `eqtb` | EngineState.eqtb.xeq_level (Rust-owned Vec) | `xeq_level`, `set_xeq_level` | MOVED session 49: C def removed; inline BSS, sized in EqtbState::default; not in format dump |
| `expand_depth` | `i32` | `scanner` | `src/globals.rs:356` C extern | `expand_depth` | def xetex/xetex-ini.c; link src/ini.rs:1824 |  |
| `expand_depth_count` | `i32` | `scanner` | `src/globals.rs:357` C extern | `expand_depth_count`, `set_expand_depth_count` | def xetex/xetex-ini.c |  |
| `cur_val_level` | `u8` | `scanner` | `src/globals.rs:360` C extern | `cur_val_level`, `set_cur_val_level` | def xetex/xetex-ini.c |  |
| `scanner_status` | `u8` | `scanner` | `src/globals.rs:364` C extern | `scanner_status`, `set_scanner_status` | def xetex/xetex-ini.c |  |
| `warning_index` | `i32` | `scanner` | `src/globals.rs:366` C extern | `warning_index`, `set_warning_index` | def xetex/xetex-ini.c |  |
| `def_ref` | `i32` | `scanner` | `src/globals.rs:368` C extern | `def_ref`, `set_def_ref` | def xetex/xetex-ini.c |  |
| `deletions_allowed` | `bool` | `errors` | `src/globals.rs:370` C extern | `deletions_allowed`, `set_deletions_allowed` | def xetex/xetex-ini.c |  |
| `long_state` | `u8` | `scanner` | `src/globals.rs:372` C extern | `long_state`, `set_long_state` | def xetex/xetex-ini.c |  |
| `first` | `i32` | `input` | `src/globals.rs:377` C extern | `first`, `set_first` | def xetex/xetex-ini.c; ref xetex/xetex-engine-interface.c |  |
| `last` | `i32` | `input` | `src/globals.rs:378` C extern | `last`, `set_last` | def xetex/xetex-ini.c |  |
| `max_buf_stack` | `i32` | `input` | `src/globals.rs:379` C extern | `max_buf_stack`, `set_max_buf_stack` | def xetex/xetex-ini.c |  |
| `buf_size` | `i32` | `input` | `src/globals.rs:380` C extern | `buf_size` | def xetex/xetex-ini.c; link src/ini.rs:1804 |  |
| `max_in_open` | `i32` | `input` | `src/globals.rs:382` C extern | `max_in_open` | def xetex/xetex-ini.c; link src/ini.rs:1808 |  |
| `source_filename_stack` | `*mut i32` | `input` | `src/globals.rs:384` C extern | `source_filename_stack`, `set_source_filename_stack` | def xetex/xetex-ini.c; link src/ini.rs:1844,src/ini.rs:3582 |  |
| `eof_seen` | `*mut bool` | `input` | `src/globals.rs:386` C extern | `eof_seen`, `set_eof_seen` | def xetex/xetex-ini.c; link src/ini.rs:1838,src/ini.rs:3576 |  |
| `input_file` | `*mut *mut libc::c_void` | `input` | `src/globals.rs:389` C extern | `input_file`, `set_input_file`, `input_file_ptr` | def xetex/xetex-ini.c; link src/ini.rs:1834,src/ini.rs:3572 |  |
| `is_in_csname` | `bool` | `scanner` | `src/globals.rs:392` C extern | `is_in_csname`, `set_is_in_csname` | def xetex/xetex-ini.c |  |
| `force_eof` | `bool` | `input` | `src/globals.rs:396` C extern | `force_eof`, `set_force_eof` | def xetex/xetex-ini.c |  |
| `used_tectonic_coda_tokens` | `bool` | `scanner` | `src/globals.rs:399` C extern | `used_tectonic_coda_tokens`, `set_used_tectonic_coda_tokens` | def xetex/xetex-ini.c |  |
| `cur_val1` | `i32` | `scanner` | `src/globals.rs:402` C extern | `cur_val1`, `set_cur_val1` | def xetex/xetex-ini.c |  |
| `radix` | `u8` | `scanner` | `src/globals.rs:404` C extern | `radix`, `set_radix` | def xetex/xetex-ini.c |  |
| `null_character` | `crate::mem_types::B16x4` | `fonts` | `EngineState.fonts.null_character` (Rust-owned) | `null_character`, `set_null_character` | MOVED session 31: C def removed |
| `dead_cycles` | `i32` | `shipout` | `src/globals.rs:410` C extern | `dead_cycles`, `set_dead_cycles` | def xetex/xetex-ini.c |  |
| `page_tail` | `i32` | `shipout` | `src/globals.rs:411` C extern | `page_tail`, `set_page_tail` | def xetex/xetex-ini.c |  |
| `last_badness` | `i32` | `shipout` | `src/globals.rs:412` C extern | `last_badness`, `set_last_badness` | def xetex/xetex-ini.c |  |
| `page_contents` | `u8` | `shipout` | `src/globals.rs:413` C extern | `page_contents`, `set_page_contents` | def xetex/xetex-ini.c |  |
| `page_so_far` | `[i32; 8]` | `shipout` | `src/globals.rs:414` C extern | `page_so_far`, `set_page_so_far` | def xetex/xetex-ini.c |  |
| `last_glue` | `i32` | `shipout` | `src/globals.rs:415` C extern | `last_glue`, `set_last_glue` | def xetex/xetex-ini.c |  |
| `last_penalty` | `i32` | `shipout` | `src/globals.rs:416` C extern | `last_penalty`, `set_last_penalty` | def xetex/xetex-ini.c |  |
| `disc_ptr` | `[i32; 4]` | `scanner` | `src/globals.rs:418` C extern | `disc_ptr`, `set_disc_ptr` | def xetex/xetex-ini.c |  |
| `last_kern` | `i32` | `shipout` | `src/globals.rs:419` C extern | `last_kern`, `set_last_kern` | def xetex/xetex-ini.c |  |
| `last_node_type` | `i32` | `shipout` | `src/globals.rs:420` C extern | `last_node_type`, `set_last_node_type` | def xetex/xetex-ini.c |  |
| `insert_penalties` | `i32` | `shipout` | `src/globals.rs:421` C extern | `insert_penalties`, `set_insert_penalties` | def xetex/xetex-ini.c |  |
| `output_active` | `bool` | `shipout` | `src/globals.rs:422` C extern | `output_active`, `set_output_active` | def xetex/xetex-ini.c |  |
| `random_seed` | `i32` | `errors` | `src/globals.rs:425` C extern | `random_seed`, `set_random_seed` | def xetex/xetex-ini.c |  |
| `pdf_last_x_pos` | `i32` | `shipout` | `src/globals.rs:428` C extern | `pdf_last_x_pos`, `set_pdf_last_x_pos` | def xetex/xetex-ini.c |  |
| `pdf_last_y_pos` | `i32` | `shipout` | `src/globals.rs:429` C extern | `pdf_last_y_pos`, `set_pdf_last_y_pos` | def xetex/xetex-ini.c |  |
| `font_area` | `*mut i32` | `fonts` | `src/globals.rs:433` C extern | `font_area_entry`, `set_font_area_entry` | def xetex/xetex-ini.c; link src/ini.rs:1898,src/ini.rs:3626 |  |
| `font_size` | `*mut i32` | `fonts` | `src/globals.rs:435` C extern | `font_size_entry`, `set_font_size_entry` | def xetex/xetex-ini.c; link src/ini.rs:1890,src/ini.rs:3618 |  |
| `font_dsize` | `*mut i32` | `fonts` | `src/globals.rs:436` C extern | `font_dsize_entry`, `set_font_dsize_entry` | def xetex/xetex-ini.c; link src/ini.rs:1892,src/ini.rs:3620 |  |
| `font_bc` | `*mut u16` | `fonts` | `src/globals.rs:438` C extern | `font_bc_entry`, `set_font_bc_entry` | def xetex/xetex-ini.c; link src/ini.rs:1900,src/ini.rs:3628 |  |
| `font_ec` | `*mut u16` | `fonts` | `src/globals.rs:439` C extern | `font_ec_entry`, `set_font_ec_entry` | def xetex/xetex-ini.c; link src/ini.rs:1902,src/ini.rs:3630 |  |
| `font_used` | `*mut bool` | `fonts` | `src/globals.rs:441` C extern | `set_font_used_entry`, `font_used_entry` | def xetex/xetex-ini.c; link src/ini.rs:1952,src/ini.rs:3562 |  |
| `bchar_label` | `*mut i32` | `fonts` | `src/globals.rs:443` C extern | `set_bchar_label_entry`, `bchar_label_entry` | def xetex/xetex-ini.c; link src/ini.rs:1910,src/ini.rs:3638 |  |
| `font_bchar` | `*mut i32` | `fonts` | `src/globals.rs:444` C extern | `set_font_bchar_entry`, `font_bchar_entry` | def xetex/xetex-ini.c; link src/ini.rs:1912,src/ini.rs:3640 |  |
| `font_false_bchar` | `*mut i32` | `fonts` | `src/globals.rs:445` C extern | `set_font_false_bchar_entry`, `font_false_bchar_entry` | def xetex/xetex-ini.c; link src/ini.rs:1914,src/ini.rs:3642 |  |
| `font_layout_engine` | `*mut *mut libc::c_void` | `fonts` | `EngineState.fonts.font_layout_engine` (Rust-owned) | `font_layout_engine_ptr`, `set_font_layout_engine_ptr` | MOVED session 39: C def + both aliases removed; C++ bridge `tt_get_font_layout_engine_entry` added |  |
| `font_mapping` | `*mut *mut libc::c_void` | `fonts` | `EngineState.fonts.font_mapping` (Rust-owned) | `font_mapping_ptr`, `set_font_mapping_ptr` | MOVED session 39: C def + both aliases removed |  |
| `font_flags` | `*mut libc::c_char` | `fonts` | `EngineState.fonts.font_flags` (Rust-owned) | `font_flags_entry`, `set_font_flags_entry` | MOVED session 39: C def + both aliases removed |  |
| `font_letter_space` | `*mut i32` | `fonts` | `EngineState.fonts.font_letter_space` (Rust-owned) | `set_font_letter_space_entry`, `font_letter_space_entry` | MOVED session 39: C def + both aliases removed |  |
| `loaded_font_mapping` | `*mut libc::c_void` | `fonts` | `EngineState.fonts.loaded_font_mapping` (Rust-owned) | `loaded_font_mapping`, `set_loaded_font_mapping` | MOVED session 29: C def removed |
| `loaded_font_flags` | `libc::c_char` | `fonts` | `EngineState.fonts.loaded_font_flags` (Rust-owned) | `loaded_font_flags`, `set_loaded_font_flags` | MOVED session 29: C def removed |
| `loaded_font_letter_space` | `i32` | `fonts` | `EngineState.fonts.loaded_font_letter_space` (Rust-owned) | `loaded_font_letter_space`, `set_loaded_font_letter_space` | MOVED session 29: C def removed |
| `mapped_text` | `*mut u16` | `fonts` | `src/globals.rs:457` C extern | `mapped_text_entry` | def xetex/xetex-ini.c |  |
| `hyphen_char` | `*mut i32` | `fonts` | `src/globals.rs:459` C extern | `hyphen_char_entry`, `set_hyphen_char_entry` | def xetex/xetex-ini.c; link src/ini.rs:1906,src/ini.rs:3634 |  |
| `skew_char` | `*mut i32` | `fonts` | `src/globals.rs:460` C extern | `skew_char_entry`, `set_skew_char_entry` | def xetex/xetex-ini.c; link src/ini.rs:1908,src/ini.rs:3636 |  |
| `xtx_ligature_present` | `bool` | `fonts` | `EngineState.fonts.xtx_ligature_present` (Rust-owned) | `xtx_ligature_present`, `set_xtx_ligature_present` | MOVED session 29: C def removed |
| `char_base` | `*mut i32` | `fonts` | `EngineState.fonts.char_base` (Rust-owned) | `char_base_entry`, `set_char_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `width_base` | `*mut i32` | `fonts` | `EngineState.fonts.width_base` (Rust-owned) | `width_base_entry`, `set_width_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `height_base` | `*mut i32` | `fonts` | `EngineState.fonts.height_base` (Rust-owned) | `height_base_entry`, `set_height_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `depth_base` | `*mut i32` | `fonts` | `EngineState.fonts.depth_base` (Rust-owned) | `depth_base_entry`, `set_depth_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `italic_base` | `*mut i32` | `fonts` | `EngineState.fonts.italic_base` (Rust-owned) | `italic_base_entry`, `set_italic_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `lig_kern_base` | `*mut i32` | `fonts` | `EngineState.fonts.lig_kern_base` (Rust-owned) | `lig_kern_base_entry`, `set_lig_kern_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `kern_base` | `*mut i32` | `fonts` | `EngineState.fonts.kern_base` (Rust-owned) | `kern_base_entry`, `set_kern_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `exten_base` | `*mut i32` | `fonts` | `EngineState.fonts.exten_base` (Rust-owned) | `exten_base_entry`, `set_exten_base_entry` | MOVED session 39: C def + both aliases removed; format dump/undump through state |  |
| `gave_char_warning_help` | `bool` | `fonts` | `EngineState.fonts.gave_char_warning_help` (Rust-owned) | `gave_char_warning_help`, `set_gave_char_warning_help` | MOVED session 29: C def removed |
| `native_font_type_flag` | `i32` | `fonts` | `EngineState.fonts.native_font_type_flag` (Rust-owned) | `native_font_type_flag`, `set_native_font_type_flag` | MOVED session 29: C def removed |
| `main_p` | `i32` | `scanner` | `src/globals.rs:479` C extern | `main_p`, `set_main_p` | def xetex/xetex-ini.c |  |
| `main_k` | `i32` | `scanner` | `src/globals.rs:480` C extern | `main_k`, `set_main_k` | def xetex/xetex-ini.c |  |
| `main_f` | `i32` | `scanner` | `src/globals.rs:482` C extern | `main_f`, `set_main_f` | def xetex/xetex-ini.c |  |
| `main_i` | `crate::mem_types::B16x4` | `scanner` | `src/globals.rs:483` C extern | none found | def xetex/xetex-ini.c |  |
| `main_j` | `crate::mem_types::B16x4` | `scanner` | `src/globals.rs:484` C extern | none found | def xetex/xetex-ini.c |  |
| `main_h` | `i32` | `scanner` | `src/globals.rs:485` C extern | `main_h`, `set_main_h` | def xetex/xetex-ini.c |  |
| `main_pp` | `i32` | `scanner` | `src/globals.rs:486` C extern | `main_pp`, `set_main_pp` | def xetex/xetex-ini.c |  |
| `main_ppp` | `i32` | `scanner` | `src/globals.rs:487` C extern | `main_ppp`, `set_main_ppp` | def xetex/xetex-ini.c |  |
| `main_s` | `i32` | `scanner` | `src/globals.rs:488` C extern | `main_s`, `set_main_s` | def xetex/xetex-ini.c |  |
| `cur_l` | `i32` | `scanner` | `src/globals.rs:490` C extern | `cur_l`, `set_cur_l` | def xetex/xetex-ini.c |  |
| `cur_r` | `i32` | `scanner` | `src/globals.rs:491` C extern | `cur_r`, `set_cur_r` | def xetex/xetex-ini.c |  |
| `cur_q` | `i32` | `scanner` | `src/globals.rs:492` C extern | `cur_q`, `set_cur_q` | def xetex/xetex-ini.c |  |
| `lig_stack` | `i32` | `scanner` | `src/globals.rs:493` C extern | `lig_stack`, `set_lig_stack` | def xetex/xetex-ini.c |  |
| `ligature_present` | `bool` | `scanner` | `src/globals.rs:494` C extern | `ligature_present`, `set_ligature_present` | def xetex/xetex-ini.c |  |
| `lft_hit` | `bool` | `scanner` | `src/globals.rs:495` C extern | `lft_hit`, `set_lft_hit` | def xetex/xetex-ini.c |  |
| `rt_hit` | `bool` | `scanner` | `src/globals.rs:496` C extern | `rt_hit`, `set_rt_hit` | def xetex/xetex-ini.c |  |
| `native_text` | `*mut u16` | `scanner` | `src/globals.rs:498` C extern | `native_text`, `set_native_text` | def xetex/xetex-ini.c; link src/ini.rs:3594 |  |
| `native_text_size` | `i32` | `scanner` | `src/globals.rs:499` C extern | `native_text_size`, `set_native_text_size` | def xetex/xetex-ini.c |  |
| `native_len` | `i32` | `scanner` | `src/globals.rs:500` C extern | `native_len`, `set_native_len` | def xetex/xetex-ini.c |  |
| `save_native_len` | `i32` | `scanner` | `src/globals.rs:501` C extern | `save_native_len`, `set_save_native_len` | def xetex/xetex-ini.c |  |
| `is_hyph` | `bool` | `hyph` | `src/globals.rs:503` C extern | `is_hyph`, `set_is_hyph` | def xetex/xetex-ini.c |  |
| `bchar` | `i32` | `scanner` | `src/globals.rs:504` C extern | `bchar`, `set_bchar` | def xetex/xetex-ini.c |  |
| `false_bchar` | `i32` | `scanner` | `src/globals.rs:505` C extern | `false_bchar`, `set_false_bchar` | def xetex/xetex-ini.c |  |
| `cancel_boundary` | `bool` | `scanner` | `src/globals.rs:506` C extern | `cancel_boundary`, `set_cancel_boundary` | def xetex/xetex-ini.c |  |
| `ins_disc` | `bool` | `scanner` | `src/globals.rs:507` C extern | `ins_disc`, `set_ins_disc` | def xetex/xetex-ini.c |  |
| `insert_src_special_auto` | `bool` | `shipout` | `src/globals.rs:509` C extern | `insert_src_special_auto` | def xetex/xetex-ini.c |  |
| `space_class` | `i32` | `scanner` | `src/globals.rs:511` C extern | `space_class`, `set_space_class` | def xetex/xetex-ini.c |  |
| `prev_class` | `i32` | `scanner` | `src/globals.rs:512` C extern | `prev_class`, `set_prev_class` | def xetex/xetex-ini.c |  |
| `hf` | `i32` | `hyph` | `src/globals.rs:514` C extern | `hf`, `set_hf` | def xetex/xetex-ini.c |  |
| `hc` | `[i32; 4099]` | `hyph` | `src/globals.rs:515` C extern | `hc_entry`, `set_hc_entry` | def xetex/xetex-ini.c |  |
| `hu` | `[i32; 4097]` | `hyph` | `src/globals.rs:516` C extern | `hu_entry`, `set_hu_entry` | def xetex/xetex-ini.c |  |
| `max_hyph_char` | `i32` | `hyph` | `src/globals.rs:517` C extern | `max_hyph_char`, `set_max_hyph_char` | def xetex/xetex-ini.c |  |
| `hyf` | `[u8; 4097]` | `hyph` | `src/globals.rs:518` C extern | `hyf_entry`, `set_hyf_entry` | def xetex/xetex-ini.c |  |
| `init_list` | `i32` | `hyph` | `src/globals.rs:519` C extern | `init_list`, `set_init_list` | def xetex/xetex-ini.c |  |
| `init_lig` | `bool` | `hyph` | `src/globals.rs:520` C extern | `init_lig`, `set_init_lig` | def xetex/xetex-ini.c |  |
| `init_lft` | `bool` | `hyph` | `src/globals.rs:521` C extern | `init_lft`, `set_init_lft` | def xetex/xetex-ini.c |  |
| `hyphen_passed` | `i16` | `hyph` | `src/globals.rs:522` C extern | `hyphen_passed`, `set_hyphen_passed` | def xetex/xetex-ini.c |  |
| `trie_trl` | `*mut i32` | `hyph` | `src/globals.rs:524` C extern | `trie_trl_entry`, `set_trie_trl_entry` | def xetex/xetex-ini.c; link src/ini.rs:1934,src/ini.rs:3662 |  |
| `trie_tro` | `*mut i32` | `hyph` | `src/globals.rs:525` C extern | `trie_tro_entry`, `set_trie_tro_entry` | def xetex/xetex-ini.c; link src/ini.rs:1936,src/ini.rs:3664 |  |
| `trie_trc` | `*mut u16` | `hyph` | `src/globals.rs:526` C extern | `trie_trc_entry`, `set_trie_trc_entry` | def xetex/xetex-ini.c; link src/ini.rs:1938,src/ini.rs:3666 |  |
| `trie_c` | `*mut u16` | `hyph` | `src/globals.rs:527` C extern | `trie_c_entry`, `set_trie_c_entry` | def xetex/xetex-ini.c; link src/ini.rs:1940 |  |
| `trie_o` | `*mut u16` | `hyph` | `src/globals.rs:528` C extern | `trie_o_entry`, `set_trie_o_entry` | def xetex/xetex-ini.c; link src/ini.rs:1942 |  |
| `trie_l` | `*mut i32` | `hyph` | `src/globals.rs:529` C extern | `trie_l_entry`, `set_trie_l_entry` | def xetex/xetex-ini.c; link src/ini.rs:1944 |  |
| `trie_r` | `*mut i32` | `hyph` | `src/globals.rs:530` C extern | `trie_r_entry`, `set_trie_r_entry` | def xetex/xetex-ini.c; link src/ini.rs:1946 |  |
| `trie_hash` | `*mut i32` | `hyph` | `src/globals.rs:531` C extern | `trie_hash_entry`, `set_trie_hash_entry` | def xetex/xetex-ini.c; link src/ini.rs:1948 |  |
| `trie_taken` | `*mut bool` | `hyph` | `src/globals.rs:532` C extern | `trie_taken_entry`, `set_trie_taken_entry` | def xetex/xetex-ini.c; link src/ini.rs:1950 |  |
| `trie_min` | `[i32; 65536]` | `hyph` | `src/globals.rs:533` C extern | `trie_min_entry`, `set_trie_min_entry` | def xetex/xetex-ini.c |  |
| `trie_max` | `i32` | `hyph` | `src/globals.rs:534` C extern | `trie_max`, `set_trie_max` | def xetex/xetex-ini.c |  |
| `trie_size` | `i32` | `hyph` | `src/globals.rs:535` C extern | `trie_size` | def xetex/xetex-ini.c; link src/ini.rs:1800 |  |
| `trie_ptr` | `i32` | `hyph` | `src/globals.rs:536` C extern | `trie_ptr`, `set_trie_ptr` | def xetex/xetex-ini.c |  |
| `hyf_distance` | `[i16; 35112]` | `hyph` | `src/globals.rs:537` C extern | `hyf_distance_entry`, `set_hyf_distance_entry` | def xetex/xetex-ini.c; link src/ini.rs:1956 |  |
| `hyf_num` | `[i16; 35112]` | `hyph` | `src/globals.rs:538` C extern | `hyf_num_entry`, `set_hyf_num_entry` | def xetex/xetex-ini.c; link src/ini.rs:1958 |  |
| `hyf_next` | `[u16; 35112]` | `hyph` | `src/globals.rs:539` C extern | `hyf_next_entry`, `set_hyf_next_entry` | def xetex/xetex-ini.c; link src/ini.rs:1960 |  |
| `op_start` | `[i32; 256]` | `hyph` | `src/globals.rs:540` C extern | `op_start_entry`, `set_op_start_entry` | def xetex/xetex-ini.c |  |
| `hyph_word` | `*mut i32` | `hyph` | `src/globals.rs:541` C extern | `hyph_word_entry`, `set_hyph_word_entry` | def xetex/xetex-ini.c; link src/ini.rs:1850,src/ini.rs:3588 |  |
| `hyph_list` | `*mut i32` | `hyph` | `src/globals.rs:542` C extern | `hyph_list_entry`, `set_hyph_list_entry` | def xetex/xetex-ini.c; link src/ini.rs:1852,src/ini.rs:3590 |  |
| `hyph_link` | `*mut u16` | `hyph` | `src/globals.rs:543` C extern | `hyph_link_entry`, `set_hyph_link_entry` | def xetex/xetex-ini.c; link src/ini.rs:1854,src/ini.rs:3592 |  |
| `hyph_count` | `i32` | `hyph` | `src/globals.rs:544` C extern | `hyph_count`, `set_hyph_count` | def xetex/xetex-ini.c |  |
| `hyph_size` | `i32` | `hyph` | `src/globals.rs:545` C extern | `hyph_size` | def xetex/xetex-ini.c; link src/ini.rs:1802 |  |
| `hyph_next` | `i32` | `hyph` | `src/globals.rs:546` C extern | `hyph_next`, `set_hyph_next` | def xetex/xetex-ini.c |  |
| `trie_used` | `[u16; 256]` | `hyph` | `src/globals.rs:547` C extern | `trie_used_entry`, `set_trie_used_entry` | def xetex/xetex-ini.c; link src/ini.rs:1962 |  |
| `trie_op_lang` | `[u8; 35112]` | `hyph` | `src/globals.rs:548` C extern | `trie_op_lang_entry`, `set_trie_op_lang_entry` | def xetex/xetex-ini.c |  |
| `trie_op_val` | `[u16; 35112]` | `hyph` | `src/globals.rs:549` C extern | `trie_op_val_entry`, `set_trie_op_val_entry` | def xetex/xetex-ini.c |  |
| `trie_op_ptr` | `i32` | `hyph` | `src/globals.rs:550` C extern | `trie_op_ptr`, `set_trie_op_ptr` | def xetex/xetex-ini.c |  |
| `max_op_used` | `u16` | `hyph` | `src/globals.rs:551` C extern | `max_op_used`, `set_max_op_used` | def xetex/xetex-ini.c |  |
| `trie_not_ready` | `bool` | `hyph` | `src/globals.rs:552` C extern | `trie_not_ready`, `set_trie_not_ready` | def xetex/xetex-ini.c |  |
| `hyph_start` | `i32` | `hyph` | `src/globals.rs:553` C extern | `hyph_start`, `set_hyph_start` | def xetex/xetex-ini.c |  |
| `hyph_index` | `i32` | `hyph` | `src/globals.rs:554` C extern | `hyph_index`, `set_hyph_index` | def xetex/xetex-ini.c |  |
| `cur_lang` | `u8` | `hyph` | `src/globals.rs:556` C extern | `cur_lang`, `set_cur_lang` | def xetex/xetex-ini.c |  |
| `insert_src_special_every_par` | `bool` | `shipout` | `src/globals.rs:558` C extern | `insert_src_special_every_par` | def xetex/xetex-ini.c |  |
| `insert_src_special_every_math` | `bool` | `shipout` | `src/globals.rs:559` C extern | `insert_src_special_every_math` | def xetex/xetex-ini.c |  |
| `insert_src_special_every_vbox` | `bool` | `shipout` | `src/globals.rs:560` C extern | `insert_src_special_every_vbox` | def xetex/xetex-ini.c |  |
| `cur_box` | `i32` | `scanner` | `src/globals.rs:562` C extern | `cur_box`, `set_cur_box` | def xetex/xetex-ini.c |  |
| `after_token` | `i32` | `scanner` | `src/globals.rs:564` C extern | `set_after_token`, `after_token` | def xetex/xetex-ini.c |  |
| `cur_order` | `u8` | `scanner` | `src/globals.rs:566` C extern | `cur_order`, `set_cur_order` | def xetex/xetex-ini.c |  |
| `cur_align` | `i32` | `scanner` | `src/globals.rs:568` C extern | `cur_align`, `set_cur_align` | def xetex/xetex-ini.c |  |
| `cur_span` | `i32` | `scanner` | `src/globals.rs:569` C extern | `cur_span`, `set_cur_span` | def xetex/xetex-ini.c |  |
| `cur_loop` | `i32` | `scanner` | `src/globals.rs:570` C extern | `cur_loop`, `set_cur_loop` | def xetex/xetex-ini.c |  |
| `align_ptr` | `i32` | `scanner` | `src/globals.rs:571` C extern | `align_ptr`, `set_align_ptr` | def xetex/xetex-ini.c |  |
| `cur_head` | `i32` | `scanner` | `src/globals.rs:572` C extern | `cur_head`, `set_cur_head` | def xetex/xetex-ini.c |  |
| `cur_tail` | `i32` | `scanner` | `src/globals.rs:573` C extern | `cur_tail_a`, `set_cur_tail_a` | def xetex/xetex-ini.c |  |
| `cur_pre_head` | `i32` | `scanner` | `src/globals.rs:574` C extern | `cur_pre_head`, `set_cur_pre_head` | def xetex/xetex-ini.c |  |
| `cur_pre_tail` | `i32` | `scanner` | `src/globals.rs:575` C extern | `cur_pre_tail`, `set_cur_pre_tail` | def xetex/xetex-ini.c |  |
| `adjust_tail` | `i32` | `scanner` | `src/globals.rs:577` C extern | `adjust_tail`, `set_adjust_tail` | def xetex/xetex-ini.c |  |
| `pre_adjust_tail` | `i32` | `scanner` | `src/globals.rs:578` C extern | `pre_adjust_tail`, `set_pre_adjust_tail` | def xetex/xetex-ini.c |  |
| `total_stretch` | `[i32; 4]` | `scanner` | `src/globals.rs:579` C extern | `total_stretch`, `set_total_stretch` | def xetex/xetex-ini.c |  |
| `total_shrink` | `[i32; 4]` | `scanner` | `src/globals.rs:580` C extern | `total_shrink`, `set_total_shrink` | def xetex/xetex-ini.c |  |
| `just_box` | `i32` | `scanner` | `src/globals.rs:581` C extern | `just_box`, `set_just_box` | def xetex/xetex-ini.c |  |
| `cur_f` | `i32` | `scanner` | `src/globals.rs:583` C extern | `cur_f`, `set_cur_f` | def xetex/xetex-ini.c |  |
| `cur_c` | `i32` | `scanner` | `src/globals.rs:584` C extern | `cur_c`, `set_cur_c` | def xetex/xetex-ini.c |  |
| `cur_i` | `crate::mem_types::B16x4` | `scanner` | `src/globals.rs:585` C extern | `cur_i`, `set_cur_i` | def xetex/xetex-ini.c |  |
| `pack_begin_line` | `i32` | `scanner` | `src/globals.rs:587` C extern | `pack_begin_line`, `set_pack_begin_line` | def xetex/xetex-ini.c |  |
| `set_box_allowed` | `bool` | `scanner` | `src/globals.rs:589` C extern | `set_box_allowed`, `set_set_box_allowed` | def xetex/xetex-ini.c |  |
| `active_width` | `[i32; 7]` | `scanner` | `src/globals.rs:592` C extern | `active_width`, `set_active_width` | def xetex/xetex-ini.c |  |
| `best_height_plus_depth` | `i32` | `scanner` | `src/globals.rs:594` C extern | `best_height_plus_depth`, `set_best_height_plus_depth` | def xetex/xetex-ini.c |  |
| `LR_ptr` | `i32` | `scanner` | `src/globals.rs:596` C extern | `lr_ptr`, `set_lr_ptr` | def xetex/xetex-ini.c |  |
| `LR_problems` | `i32` | `scanner` | `src/globals.rs:597` C extern | `lr_problems`, `set_lr_problems` | def xetex/xetex-ini.c |  |
| `cur_dir` | `i16` | `scanner` | `src/globals.rs:599` C extern | `cur_dir`, `set_cur_dir` | def xetex/xetex-ini.c |  |
| `last_leftmost_char` | `i32` | `linebreak` | `src/globals.rs:601` C extern | `last_leftmost_char`, `set_last_leftmost_char` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `last_rightmost_char` | `i32` | `linebreak` | `src/globals.rs:602` C extern | `last_rightmost_char`, `set_last_rightmost_char` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `hlist_stack` | `[i32; 513]` | `linebreak` | `src/globals.rs:604` C extern | `hlist_stack_entry`, `set_hlist_stack_entry` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `hlist_stack_level` | `i16` | `linebreak` | `src/globals.rs:605` C extern | `hlist_stack_level`, `set_hlist_stack_level` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `first_p` | `i32` | `linebreak` | `src/globals.rs:606` C extern | `first_p`, `set_first_p` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `global_prev_p` | `i32` | `linebreak` | `src/globals.rs:607` C extern | `global_prev_p`, `set_global_prev_p` | def xetex/xetex-ini.c | → `LinebreakState` (new; session 45) |
| `no_shrink_error_yet` | `bool` | `errors` | `src/globals.rs:608` C extern | `no_shrink_error_yet`, `set_no_shrink_error_yet` | def xetex/xetex-linebreak.c |  |
| `cur_p` | `i32` | `linebreak` | `src/globals.rs:609` C extern | `cur_p`, `set_cur_p` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `last_special_line` | `i32` | `linebreak` | `src/globals.rs:610` C extern | `last_special_line`, `set_last_special_line` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `first_width` | `i32` | `linebreak` | `src/globals.rs:611` C extern | `first_width`, `set_first_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `second_width` | `i32` | `linebreak` | `src/globals.rs:612` C extern | `second_width`, `set_second_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `first_indent` | `i32` | `linebreak` | `src/globals.rs:613` C extern | `first_indent`, `set_first_indent` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `second_indent` | `i32` | `linebreak` | `src/globals.rs:614` C extern | `second_indent`, `set_second_indent` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_bet` | `i32` | `linebreak` | `src/globals.rs:615` C extern | `best_bet`, `set_best_bet` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_line` | `i32` | `linebreak` | `src/globals.rs:616` C extern | `best_line`, `set_best_line` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `hn` | `i16` | `linebreak` | `src/globals.rs:617` C extern | `hn`, `set_hn` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `ha` | `i32` | `linebreak` | `src/globals.rs:618` C extern | `ha`, `set_ha` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `hb` | `i32` | `linebreak` | `src/globals.rs:619` C extern | `hb`, `set_hb` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `hyf_char` | `i32` | `hyph` | `src/globals.rs:620` C extern | `hyf_char`, `set_hyf_char` | def xetex/xetex-linebreak.c |  |
| `l_hyf` | `i32` | `hyph` | `src/globals.rs:621` C extern | `l_hyf`, `set_l_hyf` | def xetex/xetex-linebreak.c |  |
| `r_hyf` | `i32` | `hyph` | `src/globals.rs:622` C extern | `r_hyf`, `set_r_hyf` | def xetex/xetex-linebreak.c |  |
| `init_l_hyf` | `i32` | `hyph` | `src/globals.rs:623` C extern | `init_l_hyf`, `set_init_l_hyf` | def xetex/xetex-linebreak.c |  |
| `init_r_hyf` | `i32` | `hyph` | `src/globals.rs:624` C extern | `init_r_hyf`, `set_init_r_hyf` | def xetex/xetex-linebreak.c |  |
| `hyf_bchar` | `i32` | `hyph` | `src/globals.rs:625` C extern | `hyf_bchar`, `set_hyf_bchar` | def xetex/xetex-linebreak.c |  |
| `passive` | `i32` | `linebreak` | `src/globals.rs:626` C extern | `passive`, `set_passive` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `cur_active_width` | `[i32; 7]` | `linebreak` | `src/globals.rs:627` C extern | `cur_active_width`, `set_cur_active_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `background` | `[i32; 7]` | `linebreak` | `src/globals.rs:628` C extern | `background`, `set_background` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `break_width` | `[i32; 7]` | `linebreak` | `src/globals.rs:629` C extern | `break_width`, `set_break_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_place` | `[i32; 4]` | `linebreak` | `src/globals.rs:630` C extern | `best_place`, `set_best_place` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_pl_line` | `[i32; 4]` | `linebreak` | `src/globals.rs:631` C extern | `best_pl_line`, `set_best_pl_line` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `disc_width` | `i32` | `linebreak` | `src/globals.rs:632` C extern | `disc_width`, `set_disc_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `final_pass` | `bool` | `linebreak` | `src/globals.rs:633` C extern | `final_pass`, `set_final_pass` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `threshold` | `i32` | `linebreak` | `src/globals.rs:634` C extern | `threshold`, `set_threshold` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `minimal_demerits` | `[i32; 4]` | `linebreak` | `src/globals.rs:635` C extern | `minimal_demerits`, `set_minimal_demerits` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `minimum_demerits` | `i32` | `linebreak` | `src/globals.rs:636` C extern | `minimum_demerits`, `set_minimum_demerits` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `easy_line` | `i32` | `linebreak` | `src/globals.rs:637` C extern | `easy_line`, `set_easy_line` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `do_last_line_fit` | `bool` | `linebreak` | `src/globals.rs:638` C extern | `do_last_line_fit`, `set_do_last_line_fit` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `active_node_size` | `i16` | `linebreak` | `src/globals.rs:639` C extern | `active_node_size`, `set_active_node_size` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `fill_width` | `[i32; 3]` | `linebreak` | `src/globals.rs:640` C extern | `fill_width`, `set_fill_width` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_pl_short` | `[i32; 4]` | `linebreak` | `src/globals.rs:641` C extern | `best_pl_short`, `set_best_pl_short` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `best_pl_glue` | `[i32; 4]` | `linebreak` | `src/globals.rs:642` C extern | `best_pl_glue`, `set_best_pl_glue` | def xetex/xetex-linebreak.c | → `LinebreakState` (new; session 45) |
| `font_in_short_display` | `i32` | `errors` | `src/globals.rs:644` C extern | `font_in_short_display`, `set_font_in_short_display` | def xetex/xetex-ini.c |  |
| `depth_threshold` | `i32` | `errors` | `src/globals.rs:646` C extern | `depth_threshold`, `set_depth_threshold` | def xetex/xetex-ini.c |  |
| `breadth_max` | `i32` | `errors` | `src/globals.rs:647` C extern | `breadth_max`, `set_breadth_max` | def xetex/xetex-ini.c |  |
| `total_pages` | `i32` | `shipout` | EngineState.shipout.total_pages (Rust-owned) | `total_pages`, `set_total_pages` | MOVED session 47: C def removed |
| `max_v` | `i32` | `shipout` | EngineState.shipout.max_v (Rust-owned) | `max_v`, `set_max_v` | MOVED session 47: C def removed |
| `max_h` | `i32` | `shipout` | EngineState.shipout.max_h (Rust-owned) | `max_h`, `set_max_h` | MOVED session 47: C def removed (xetex-math.c "ref" was a `make_left_right` parameter name, not the global) |
| `max_push` | `i32` | `shipout` | EngineState.shipout.max_push (Rust-owned) | `max_push`, `set_max_push` | MOVED session 47: C def removed |
| `last_bop` | `i32` | `shipout` | EngineState.shipout.last_bop (Rust-owned) | `last_bop`, `set_last_bop` | MOVED session 47: C def removed |
| `doing_leaders` | `bool` | `shipout` | EngineState.shipout.doing_leaders (Rust-owned) | `doing_leaders`, `set_doing_leaders` | MOVED session 47: C def removed |
| `rule_ht` | `i32` | `shipout` | EngineState.shipout.rule_ht (Rust-owned) | `rule_ht`, `set_rule_ht` | MOVED session 47: C def removed |
| `rule_dp` | `i32` | `shipout` | EngineState.shipout.rule_dp (Rust-owned) | `rule_dp`, `set_rule_dp` | MOVED session 47: C def removed |
| `rule_wd` | `i32` | `shipout` | EngineState.shipout.rule_wd (Rust-owned) | `rule_wd`, `set_rule_wd` | MOVED session 47: C def removed |
| `cur_h` | `i32` | `shipout` | EngineState.shipout.cur_h (Rust-owned) | `cur_h`, `set_cur_h` | MOVED session 47: C def removed |
| `cur_v` | `i32` | `shipout` | EngineState.shipout.cur_v (Rust-owned) | `cur_v`, `set_cur_v` | MOVED session 47: C def removed |
| `cur_page_width` | `i32` | `shipout` | EngineState.shipout.cur_page_width (Rust-owned) | `cur_page_width`, `set_cur_page_width` | MOVED session 47: C def removed |
| `cur_page_height` | `i32` | `shipout` | EngineState.shipout.cur_page_height (Rust-owned) | `cur_page_height`, `set_cur_page_height` | MOVED session 47: C def removed |
| `cur_h_offset` | `i32` | `shipout` | EngineState.shipout.cur_h_offset (Rust-owned) | `cur_h_offset`, `set_cur_h_offset` | MOVED session 47: C def removed |
| `cur_v_offset` | `i32` | `shipout` | EngineState.shipout.cur_v_offset (Rust-owned) | `cur_v_offset`, `set_cur_v_offset` | MOVED session 47: C def removed |
| `write_loc` | `i32` | `shipout` | EngineState.shipout.write_loc (Rust-owned) | `write_loc`, `set_write_loc` | MOVED session 47: C def removed |
| `cur_mlist` | `i32` | `scanner` | EngineState.scanner.cur_mlist (Rust-owned) | `cur_mlist`, `set_cur_mlist` | MOVED session 48: C def removed |
| `cur_style` | `i16` | `scanner` | EngineState.scanner.cur_style (Rust-owned) | `cur_style`, `set_cur_style` | MOVED session 48: C def removed |
| `cur_size` | `i32` | `scanner` | EngineState.scanner.cur_size (Rust-owned) | `cur_size`, `set_cur_size` | MOVED session 48: C def removed |
| `cur_mu` | `i32` | `scanner` | EngineState.scanner.cur_mu (Rust-owned) | `cur_mu`, `set_cur_mu` | MOVED session 48: C def removed |
| `mlist_penalties` | `bool` | `scanner` | EngineState.scanner.mlist_penalties (Rust-owned) | `mlist_penalties`, `set_mlist_penalties` | MOVED session 48: C def removed |
| `format_ident` | `i32` | `strings` | `src/globals.rs:679` C extern | `format_ident`, `set_format_ident` | def xetex/xetex-ini.c |  |
| `edit_name_start` | `i32` | `strings` | `src/globals.rs:681` C extern | `edit_name_start`, `set_edit_name_start` | def xetex/xetex-ini.c |  |
| `empty` | `crate::mem_types::B32x2` | `scanner` | `src/globals.rs:683` C extern | `set_empty_b32s0`, `set_empty_b32s1` | def xetex/xetex-ini.c; ref xetex/xetex-xetex0.c |  |

## Promoted Rust statics

| name | type | cluster | defined where | accessors | C/link status | notes |
|---|---|---|---|---|---|---|
| `_trie_op_hash_array` | `[i32; TRIE_OP_HASH_SIZE]` | `hyph` | `src/ini.rs:77` Rust static | n/a | extern xetex/xetex-ini.c |  |
| `name_of_input_file` | `*mut libc::c_char` | `input` | `src/io.rs:178` Rust static | n/a | ref xetex/xetex-io.c |  |
| `abspath_of_input_file` | `[libc::c_char; 1024]` | `input` | `src/io.rs:182` Rust static | n/a | ref xetex/xetex-io.c |  |
| `dvi_file` | `usize` | `shipout` | `src/shipout.rs:54` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `output_file_name` | `i32` | `shipout` | `src/shipout.rs:57` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_buf` | `*mut u8` | `shipout` | `src/shipout.rs:60` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_limit` | `i32` | `shipout` | `src/shipout.rs:63` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_ptr` | `i32` | `shipout` | `src/shipout.rs:66` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_offset` | `i32` | `shipout` | `src/shipout.rs:69` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_gone` | `i32` | `shipout` | `src/shipout.rs:72` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `down_ptr` | `i32` | `shipout` | `src/shipout.rs:75` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `right_ptr` | `i32` | `shipout` | `src/shipout.rs:78` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `cur_s` | `i32` | `shipout` | `src/shipout.rs:81` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_h` | `i32` | `shipout` | `src/shipout.rs:85` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_v` | `i32` | `shipout` | `src/shipout.rs:88` Rust static | n/a | extern xetex/xetex-shipout.c |  |
| `dvi_f` | `i32` | `shipout` | `src/shipout.rs:93` Rust static | n/a | extern xetex/xetex-shipout.c |  |

## Private module-local statics

These are not C-symbol globals from `globals.rs`, but they are still mutable engine state and should be folded into Stage 2 workstreams when their owning modules are idiomatized.

| name | type | cluster | defined where | notes |
|---|---|---|---|---|
| `SAVED_MAPPING_NAME` | `*mut c_char` | `fonts` | `src/ext.rs:50` private static |  |
| `BRK_ITER` | `*mut c_void` | `fonts` | `src/ext.rs:272` private static |  |
| `BRK_LOCALE_STR_NUM` | `i32` | `fonts` | `src/ext.rs:273` private static |  |
| `XDV_BUF_SIZE` | `c_int` | `fonts` | `src/ext.rs:274` private static |  |
| `APPLY_MAPPING_OUT_LENGTH` | `u32` | `fonts` | `src/ext.rs:275` private static |  |
| `STDIN_UFILE` | `UFile` | `input` | `src/ini.rs:111` private static |  |
| `NAME16LEN` | `i32` | `input` | `src/io.rs:186` private static |  |
| `BYTE_BUFFER` | `*mut libc::c_char` | `input` | `src/io.rs:342` private static |  |
| `UTF32_BUF` | `*mut u32` | `input` | `src/io.rs:344` private static |  |
| `NORMALIZERS` | `[*mut libc::c_void; 2]` | `input` | `src/io.rs:346` private static |  |
| `BEST_PAGE_BREAK` | `i32` | `shipout` | `src/pagebuilder.rs:19` private static | → `ShipoutState` (fold; session 46) |
| `BEST_SIZE` | `i32` | `shipout` | `src/pagebuilder.rs:20` private static | → `ShipoutState` (fold; session 46) |
| `LEAST_PAGE_COST` | `i32` | `shipout` | `src/pagebuilder.rs:21` private static | → `ShipoutState` (fold; session 46) |
| `PAGE_MAX_DEPTH` | `i32` | `shipout` | `src/pagebuilder.rs:22` private static | → `ShipoutState` (fold; session 46) |
| `SYNCTEX_CTXT` | `SynctexCtxt` | `synctex` | `src/synctex.rs:82` private static | → `EngineState.synctex: SynctexCtxt` (session 46) |
| `LAST_SOURCE_NAME` | `*mut libc::c_char` | `texmf` | `src/texmfmp.rs:51` private static | → `TexmfState` (new; session 46) |
| `LAST_LINENO` | `i32` | `texmf` | `src/texmfmp.rs:52` private static | → `TexmfState` (new; session 46) |
| `START_TIME_STR` | `[u8; 30]` | `texmf` | `src/texmfmp.rs:54` private static | → `TexmfState` (new; session 46) |

## Deferred newtype worklist

Stage 1 collapsed every semantic C type into raw `i32`/`u8`. Phase-B
cluster moves keep that raw representation (and the accessor signature)
on purpose — see `01-state-and-memory.md` §4. This table is the backlog
the §4(b) newtype sweep must clear so that type safety is *recovered*,
not silently lost when a field becomes a bare integer inside
`EngineState`.

**Rule:** every cluster migration must add a row here for each field it
moves whose declared C type (in `xetex-xetexd.h` / `xetex-core.h`) is a
semantic typedef or enum rather than a plain numeric `int32_t`/`bool`.
Genuinely-numeric counters (`dead_cycles`, `epochseconds`,
`insert_penalties`, …) do not need rows.

Status: `pending` until the newtype exists and the accessor returns it.

| field | cluster | original C type | target newtype | migrated in | status |
|---|---|---|---|---|---|
| `selector` | `shipout` | `selector_t` (enum, 0–21) | `Selector` | session 7 | pending |
| `page_contents` | `shipout` | `unsigned char` (PC_* state code) | `PageContents` | session 7 | pending |
| `last_node_type` | `shipout` | `int32_t` (node-type code) | `NodeType`/`Option<NodeType>` | session 7 | pending |
| `last_glue` | `shipout` | `int32_t` (glue node ptr) | `MemPtr`/`Halfword` | session 7 | pending |
| `last_kern` | `shipout` | `scaled_t` | `Scaled` | session 7 | pending |
| `page_so_far` | `shipout` | `scaled_t[8]` | `[Scaled; 8]` | session 7 | pending |
| `old_setting` | `shipout` | `unsigned char` (saved `selector_t`) | `Selector` | session 7 | pending |
| `history` | `errors` | `tt_history_t` (enum) | `History` | session 5 | pending |
| `interaction` | `errors` | `unsigned char` (interaction mode) | `InteractionMode` | session 5 | pending |
| `cur_name` | `scanner` | `str_number` | `StrNumber` | session 8 | pending |
| `cur_area` | `scanner` | `str_number` | `StrNumber` | session 8 | pending |
| `cur_ext` | `scanner` | `str_number` | `StrNumber` | session 8 | pending |
| `area_delimiter` | `scanner` | `pool_pointer` | `PoolPtr` | session 8 | pending |
| `ext_delimiter` | `scanner` | `pool_pointer` | `PoolPtr` | session 8 | pending |
| `cur_group` | `save_stack` | `group_code` (enum) | `GroupCode` | session 9 | pending |
| `grp_stack[]` | `input` | `save_pointer` | `SavePointer`/`Scaled`-style index | session 13 | pending |
| `source_filename_stack[]` | `input` | `str_number` | `StrNumber` | session 13 | pending |
| `full_source_filename_stack[]` | `input` | `str_number` | `StrNumber` | session 13 | pending |
| `buffer[]` | `input` | `UnicodeScalar` (`int32_t` typedef, a Unicode scalar value) | `UnicodeScalar` | session 15 | pending |
| `write_file[]` | `shipout` | `rust_output_handle_t` (`uintptr_t` typedef; nullable output-stream handle, 0 = none) | `OutputHandle`/`Option<OutputId>` | session 19 | pending |
| `radix` | `scanner` | `small_number` (`typedef short`; scanner radix 8/10/16, stored as `u8` in the bridge) | `SmallNumber`/`Radix` | session 21 | pending |
| `rust_stdout` | `shipout` | `rust_output_handle_t` (`uintptr_t` typedef; stdout stream handle) | `OutputHandle` | session 23 | pending |
| `log_file` | `shipout` | `rust_output_handle_t` (`uintptr_t` typedef; log stream handle, 0 = none) | `OutputHandle`/`Option<OutputId>` | session 23 | pending |
| `str_ptr` | `strings` | `str_number` | `StrNumber` | session 24 | pending |
| `init_str_ptr` | `strings` | `str_number` | `StrNumber` | session 24 | pending |
| `init_pool_ptr` | `strings` | `pool_pointer` | `PoolPtr` | session 24 | pending |
| `job_name` | `strings` | `str_number` | `StrNumber` | session 24 | pending |
| `texmf_log_name` | `strings` | `str_number` | `StrNumber` | session 24 | pending |
| `pool_ptr` | `strings` | `pool_pointer` | `PoolPtr` | session 24 | pending |
| `format_ident` | `strings` | `str_number` | `StrNumber` | session 24 | pending |
| `edit_name_start` | `strings` | `pool_pointer` | `PoolPtr` | session 24 | pending |
| `hf` | `hyph` | `internal_font_number` | `FontId` | session 25 | pending |
| `hyphen_passed` | `hyph` | `small_number` | `SmallNumber` | session 25 | pending |
| `trie_max` | `hyph` | `trie_pointer` | `TrieIndex` | session 25 | pending |
| `trie_ptr` | `hyph` | `trie_pointer` | `TrieIndex` | session 25 | pending |
| `max_op_used` | `hyph` | `trie_opcode` | `TrieOpcode` | session 25 | pending |
| `hyph_start` | `hyph` | `trie_pointer` | `TrieIndex` | session 25 | pending |
| `hyph_index` | `hyph` | `trie_pointer` | `TrieIndex` | session 25 | pending |
| `trie_min[]` | `hyph` | `trie_pointer[65536]` | `Vec<TrieIndex>` | session 26 | pending |
| `hyf_distance[]` | `hyph` | `small_number[TRIE_OP_SIZE + 1]` | `Vec<SmallNumber>` | session 26 | pending |
| `hyf_num[]` | `hyph` | `small_number[TRIE_OP_SIZE + 1]` | `Vec<SmallNumber>` | session 26 | pending |
| `hyf_next[]` | `hyph` | `trie_opcode[TRIE_OP_SIZE + 1]` | `Vec<TrieOpcode>` | session 26 | pending |
| `trie_used[]` | `hyph` | `trie_opcode[256]` | `Vec<TrieOpcode>` | session 26 | pending |
| `trie_op_val[]` | `hyph` | `trie_opcode[TRIE_OP_SIZE + 1]` | `Vec<TrieOpcode>` | session 26 | pending |
| `trie_trl[]` | `hyph` | `trie_pointer *` | `Vec<TrieIndex>` | session 27 | pending |
| `trie_tro[]` | `hyph` | `trie_pointer *` | `Vec<TrieIndex>` | session 27 | pending |
| `trie_c[]` | `hyph` | `packed_UTF16_code *` | `Vec<PackedUtf16Code>` | session 27 | pending |
| `trie_o[]` | `hyph` | `trie_opcode *` | `Vec<TrieOpcode>` | session 27 | pending |
| `trie_l[]` | `hyph` | `trie_pointer *` | `Vec<TrieIndex>` | session 27 | pending |
| `trie_r[]` | `hyph` | `trie_pointer *` | `Vec<TrieIndex>` | session 27 | pending |
| `trie_hash[]` | `hyph` | `trie_pointer *` | `Vec<TrieIndex>` | session 27 | pending |
| `hyph_word[]` | `hyph` | `str_number *` | `Vec<StrNumber>` | session 28 | pending |
| `hyph_link[]` | `hyph` | `hyph_pointer *` | `Vec<HyphPointer>` | session 28 | pending |
| `loaded_font_letter_space` | `fonts` | `scaled_t` | `Scaled` | session 29 | pending |
| `font_size[]` | `fonts` | `scaled_t *` | `Vec<Scaled>` | session 33 | pending |
| `font_dsize[]` | `fonts` | `scaled_t *` | `Vec<Scaled>` | session 33 | pending |
| `font_params[]` | `fonts` | `font_index *` | `Vec<FontIndex>` | session 34 | pending |
| `font_name[]` | `fonts` | `str_number *` | `Vec<StrNumber>` | session 35 | pending |
| `font_area[]` | `fonts` | `str_number *` | `Vec<StrNumber>` | session 35 | pending |
| `font_bc[]` | `fonts` | `UTF16_code *` | `Vec<Utf16Code>` | session 36 | pending |
| `font_ec[]` | `fonts` | `UTF16_code *` | `Vec<Utf16Code>` | session 36 | pending |
| `bchar_label[]` | `fonts` | `font_index *` | `Vec<FontIndex>` | session 37 | pending |
| `font_bchar[]` | `fonts` | `nine_bits *` | `Vec<NineBits>` | session 37 | pending |
| `font_false_bchar[]` | `fonts` | `nine_bits *` | `Vec<NineBits>` | session 37 | pending |
| `font_glue[]` | `fonts` | `int32_t *` (nullable glue-spec mem pointer; checked undump vs `lo_mem_max`) | `Vec<MemPtr>` | session 38 | pending |
| `fmem_ptr` | `fonts` | `font_index` (semantic typedef for `int32_t`, font-info write pointer) | `FontIndex`/`FmemPtr` | session 39 | pending |
| `font_ptr` | `fonts` | `internal_font_number` (semantic typedef for `int32_t`, current font count) | `InternalFontNumber`/`FontId` | session 39 | pending |
| `font_letter_space[]` | `fonts` | `scaled_t *` (per-font letter spacing; not format-serialized) | `Vec<Scaled>` | session 39 | pending |
| `cur_if` | `scanner` | `small_number` (`typedef short`; current `\if` type code, same typedef as `radix`) | `SmallNumber` | session 41 | pending |
| `cur_active_width[]` | `linebreak` | `scaled_t[7]` | `[Scaled; 7]` | session 45 | pending |
| `background[]` | `linebreak` | `scaled_t[7]` | `[Scaled; 7]` | session 45 | pending |
| `break_width[]` | `linebreak` | `scaled_t[7]` | `[Scaled; 7]` | session 45 | pending |
| `disc_width` | `linebreak` | `scaled_t` | `Scaled` | session 45 | pending |
| `first_width` | `linebreak` | `scaled_t` | `Scaled` | session 45 | pending |
| `second_width` | `linebreak` | `scaled_t` | `Scaled` | session 45 | pending |
| `first_indent` | `linebreak` | `scaled_t` | `Scaled` | session 45 | pending |
| `second_indent` | `linebreak` | `scaled_t` | `Scaled` | session 45 | pending |
| `fill_width[]` | `linebreak` | `scaled_t[3]` | `[Scaled; 3]` | session 45 | pending |
| `best_pl_short[]` | `linebreak` | `scaled_t[4]` | `[Scaled; 4]` | session 45 | pending |
| `best_pl_glue[]` | `linebreak` | `scaled_t[4]` | `[Scaled; 4]` | session 45 | pending |
| `hn` | `linebreak` | `small_number` (`typedef short`; hyphenation char count) | `SmallNumber` | session 45 | pending |
| `active_node_size` | `linebreak` | `small_number` (`typedef short`; active-node word size) | `SmallNumber` | session 45 | pending |
| `last_source_name` | `texmf` | `char *` — **owned C string** (`gettexstring`/`libc::free`), not a numeric typedef; ownership debt, not a width debt | `Option<CString>` | session 46 | pending |
| `synctex.root_name` | `synctex` | `char *` — **owned C string** (`xstrdup`/`get_current_name`/`mfree`), ownership debt, not a width debt | `Option<CString>` | session 46 | pending |
| `synctex.file` | `synctex` | stream handle (`usize`; 0 = INVALID_HANDLE), same shape as `rust_stdout`/`log_file` | `OutputHandle`/`Option<OutputId>` | session 46 | pending |
| `max_v` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `max_h` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `rule_ht` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `rule_dp` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `rule_wd` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_h` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_v` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_page_width` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_page_height` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_h_offset` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_v_offset` | `shipout` | `scaled_t` | `Scaled` | session 47 | pending |
| `cur_style` | `scanner` | `small_number` (`typedef short`; current math style) | `SmallNumber` | session 48 | pending |
| `cur_mu` | `scanner` | `scaled_t` (math unit `mu`) | `Scaled` | session 48 | pending |


(`scanner` session-48 math-scratch move: `cur_mlist` is declared plain
`int32_t` in `xetex-math.c` (mlist mem pointer; WEB pointer typedef erased in
Stage 1 — a `MemPtr` pointer-taxonomy-sweep candidate, not a typedef-loss row),
`cur_size` is a plain `int32_t` size code, and `mlist_penalties` is a plain
`bool` — none need a row. `cur_style` (`small_number`) and `cur_mu` (`scaled_t`)
are the only debts. None are in the format dump.)

(`eqtb` session-49 inline-array move: `prim` elements are `B32x2` (the
primitive-table link/text halves — already the faithful struct, no semantic
typedef collapsed) and `xeq_level` elements are `u16` save levels
(`quarterword`, a plain count) — neither is a width/typedef debt, so no rows.
Both were inline C BSS arrays (no `malloc`), so they are sized in
`EqtbState::default` rather than allocated in `ini.rs`; `prim` is format-dumped
but only through its `prim_s0`/`prim_s1` accessors, so the bridge is transparent;
`xeq_level` is not in the format dump. The malloc'd `eqtb`/`hash` arrays — with
the raw `dump_slice` reconciliation and the `yhash` offset-pointer trick — remain
for dedicated sessions.)

(`shipout` session-47 DVI-scratch move: `total_pages`, `max_push`, `last_bop`
are plain `int32_t` counters/byte-offsets, `doing_leaders` is a plain `bool`,
and `write_loc` is a plain `int32_t` eqtb location (declared `int32_t` in the C
source, the WEB pointer typedef already erased in Stage 1 — a `MemPtr` candidate
for the later pointer-taxonomy sweep, not a typedef-loss row) — none need a row.
The 11 `scaled_t` position/dimension fields above are the only debts. `write_loc`
is in the format dump but serialized/deserialized through its accessor, so the
bridge is transparent — no ownership reconciliation needed.)

(`synctex` session-46 move: `SYNCTEX_CTXT` was already a Stage-1-translated
private Rust struct, *not* a C-header global, so its scalar fields moved into
`EngineState.synctex` verbatim with unchanged Rust types — no typedef-loss debt.
The C `synctex_ctxt_t` declared `curh`/`curv`/`lastv`/`magnification`/`unit`/
`total_length` as plain `int` (sp positions, not the `scaled_t` typedef) and
`tag`/`line`/`node`/`count`/`form_depth` as plain ints, so only the two rows
above are debts: `root_name` (owned C string) and `file` (output handle).
`node` is a mem pointer (pointer-taxonomy-sweep candidate, no typedef row);
`recorder` is a fn pointer; the flags are plain `bool`.)

(`linebreak` session-45 move: the `int32_t` mem-pointer fields — `cur_p`,
`first_p`, `global_prev_p`, `passive`, `best_bet`, `best_place[]`,
`best_pl_line[]`, `ha`, `hb` — are declared plain `int32_t` in the C
source (the WEB `pointer` typedef was already erased in Stage 1), so they
get **no** typedef-loss row; they are `MemPtr`/`Halfword` candidates for
the later pointer-taxonomy sweep, same disposition as `if_stack`'s
`cond_ptr` halfwords. `last_special_line`/`best_line` hold line numbers,
`threshold`/`minimum_demerits`/`minimal_demerits[]`/`easy_line` are plain
demerit/line integers, `final_pass`/`do_last_line_fit` are plain `bool`,
`hlist_stack[]`/`last_leftmost_char`/`last_rightmost_char` are plain
`int32_t`, and `hlist_stack_level` is a plain `short` stack pointer — none
need a row.)

(`input` session-13 move of the line/group stacks: `line_stack` and
`if_stack` are plain `int32_t *` and need no row — `line_stack` holds
line numbers, `if_stack` holds `cond_ptr` halfwords that become a `MemPtr`
in the later pointer-taxonomy sweep, not a typedef-loss row.)

(`save_stack` session-9 move: `save_ptr`/`max_save_stack`/`cur_boundary`
are plain `int32_t` and `cur_level` is a `uint16_t` nesting count, so
only `cur_group` is a debt.)

(`file_name_quote_char` is `UTF16_code`/`u16` — a genuine character value,
not a semantic typedef, so it needs no row; `quoted_filename`,
`name_in_progress`, `stop_at_space` are plain `bool` flags.)

(`texmf` session-46 move: `last_lineno` is a plain `int32_t` line number and
`start_time_str` is a plain `[u8; 30]` byte buffer — neither is a semantic
typedef, so neither needs a row. `last_source_name` is the only debt, and it is
an *ownership* debt (owned C string → `Option<CString>`), not a width/typedef
debt.)

Earlier clusters (`config`) were re-checked: their migrated fields are
plain `int32_t`/`bool` flags with no semantic C type, so they need no
rows. The `errors` rows above were added retroactively when this rule
was introduced (session 7 review).
