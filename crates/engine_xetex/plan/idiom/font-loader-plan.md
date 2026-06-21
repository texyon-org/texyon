# Font-loader threading plan (the `prefixed_command` blocker)

> Status: **READY for execution** (recon session 216). The companion to
> `scanner-scc-plan.md`: a de-risked, bottom-up threading order for the
> font-loading subsystem, the sole remaining blocker for `prefixed_command_in`
> (session-216 finding — see `STATUS.md`/`SESSION_LOG.md` session 216).

## Why this is the critical path

`prefixed_command`'s `DEF_FONT` arm calls `new_font` (main_control.rs:3884),
which is the **only** un-threaded function-level callee of `prefixed_command`
(every other callee has an `_in` core or is a trivial `with_state` field
accessor rewritten inline). So:

```
prefixed_command_in  →  new_font_in  →  read_font_info_in
                                          ├─ read_font_info_tfm   (TFM path)
                                          │    └─ load_tfm_font_mapping / check_for_tfm_font_mapping  (ext.rs, FFI-only, no state)
                                          └─ load_native_font     (native path)
                                               └─ find_native_font  (ext.rs, with_state via accessors)
```

and once `new_font_in` exists, `prefixed_command_in` is a mechanical
single-borrow pass → `do_assignments_in` (trivial) → the `fin_align`/`align_peek`/
`init_align`/`fin_row` cycle → `handle_right_brace_in` → the dispatcher.

## Sizes (recon)

| fn | file | LOC | path | validatable by |
|---|---|---:|---|---|
| `read_font_info` | fonts.rs:343 | 170 | hub | dashboard (TFM) |
| `read_font_info_tfm` | fonts.rs:534 | 381 | TFM | **dashboard** (TRIP/goldens use cmr10) |
| `load_native_font` | fonts.rs:1002 | 149 | native | **corpus-only** (no native font in dashboard) |
| `find_native_font` | ext.rs:1049 | 144 | native | corpus-only |
| `new_font` | main_control.rs:3884 | 153 | hub | dashboard (TFM) + corpus (native) |
| `check_for_tfm_font_mapping` | ext.rs:665 | 13 | — | FFI-only, **no state** (already borrow-safe) |
| `load_tfm_font_mapping` | ext.rs:681 | 11 | — | FFI-only, **no state** (already borrow-safe) |

## FFI re-entry analysis (the key de-risking — there is NO OT-math-style wall)

The session-206 OT-math wall was C++ (`xetex-XeTeXOTMath.cpp`) calling back into
Rust `tt_get_font_*` accessors. For the font loaders:

- **`get_ot_math_constant(font_ptr, k)`** (load_native_font tail) DOES route
  through `tt_get_font_{size,area,layout_engine}_entry`, **but those are now the
  borrow-free mirror callbacks** (session 207, `globals::font_ffi_mirror`). The
  mirror is refreshed only at the 3 font-table realloc sites; `load_native_font`
  does in-place writes (font_ptr ≤ font_max, no realloc), so the mirror is live.
  **Safe under a held borrow.** Same for `isOpenTypeMathFont`/`get_loaded_font_design_size`
  (pure C, no Rust callback).
- **`find_native_font`/`release_font_engine`** (ext.rs) are Rust + xetex_layout
  FFI. They have **0 literal `with_state`** but reach state through globals
  accessors (`name_of_file`/`set_name_of_file`/`set_name_length`/`set_loaded_font_*`),
  so they ARE `with_state`-bearing and must be threaded before `load_native_font`
  can hold a borrow across them. No re-entrant FFI callback to `with_state`,
  though — they only touch the `loaded_font_*` + `name_of_file` scratch globals.
- **`read_font_info_tfm`** has **0 literal `with_state`**; all its state access is
  through globals accessors (font-table setters, `font_info` pushes, pool writes,
  TFM read via `ttstub_input_*` FFI which is state-free). Pure mechanical
  accessor→`st.fonts`/`st.strings`/`st.eqtb` conversion.
- **`init_native_font_tables`** already takes `(&mut FontState, &EqtbState)`
  (session 129) — call it directly under the borrow.
- **`new_native_character`** has `new_native_character_in(&mut EngineState,…)`
  (session 187, re-entry-free) — call under the borrow.

**Conclusion: no hard wall. The whole subsystem is straight accessor-conversion,
threadable bottom-up.** The only subtlety is the mirror (already handled).

## Bottom-up execution order (one commit per row, dashboard or corpus per the table)

1. **`find_native_font` + `release_font_engine`** → `_in(&mut EngineState)` (or
   `&mut FontState`+`&mut StringsState` if it only needs those): convert the
   `loaded_font_*`/`name_of_file`/`name_length` accessors to `st.*`. Keep thin
   wrappers (other callers: `read_font_info`, maybe `aat`/`ot` code). Native →
   **validate by full corpus** (doc-count == baseline, no NOXDV/NOLOG —
   `[[corpus-doccount-regression-check]]`).
2. **`read_font_info_tfm` → `read_font_info_tfm_in`**: mechanical accessor sweep
   (`thread_fn.py --apply` handles the regular `mem`/`eqtb` ones; the font-table
   and pool accessors are hand-wired). TFM → **dashboard validates** (cmr10).
   Cold `overflow`/`_tt_abort`/`error` ride `SccAbort`/a font-local sentinel.
3. **`load_native_font → load_native_font_in`**: now that `find_native_font_in`
   exists, hold one borrow; `get_ot_math_constant` is mirror-safe; call
   `new_native_character_in`/`init_native_font_tables`/`free_node`/`make_string`/
   `push_font_info_b32s1` (thread this tiny scan.rs:2559 helper to `&mut FontState`)
   under it. Native → **full corpus**.
4. **`read_font_info → read_font_info_in`**: thin over the two threaded loaders +
   `check_for_tfm_font_mapping_in`/`load_tfm_font_mapping` (the latter two are
   already state-free; just pass `st`'s `name_of_file`). dashboard + corpus.
5. **`new_font → new_font_in`** (main_control.rs): single borrow over
   `read_font_info_in` + `get_r_token_in`/`maketexstring_in`/`scan_optional_equals_in`/
   `scan_file_name_in`/`scan_keyword_in`/`scan_dimen_in`/`scan_int_in`, the eqtb
   defines (`eq_define`/`geq_define`), `str_eq_str`/`append_str`/`make_string`,
   `xn_over_d` (use a `_in` form or `&mut errors`). Recoverable errors via
   `mc_propagate_error`; pool overflow via `InputAbort::Overflow`. dashboard +
   corpus.
6. **`prefixed_command → prefixed_command_in`** (main_control.rs, ~1000 lines):
   now mechanical — every callee is threaded. The remaining `with_state` field
   accessors (`eqtb_b32s1`→`st.eqtb.data[..].b32.s1`, `mem_b32s*`→`st.mem.*`,
   `def_ref`→`st.scanner.def_ref`, `avail`→`st.mem.avail`, `font_area_entry`→
   `st.fonts.*`, `set_help_*`→`st.errors.*`, `after_token`/`set_after_token`,
   `set_box_allowed`, `set_hyphen_char_entry`/`set_skew_char_entry`/
   `set_font_info_b32s1`→`st.fonts.*`, `in_initex_mode`→config) convert inline.
   Returns `Result<(), MainControlAbort>`. **This is the prize.**
7. **`do_assignments → do_assignments_in`** (trivial: `get_x_token_in` loop +
   `prefixed_command_in`), then the alignment cycle per
   `[[handle-right-brace-blocked-on-alignment]]`.

## Validation rule (do not skip)

The dashboard has **no native font**, so steps 1/3/4/5's native path is invisible
to it — a nest-panic shows only as **NOXDV/NOLOG** in the full corpus (the
session-187 lesson, `[[corpus-doccount-regression-check]]`). After every step
that touches the native path, run the full corpus and compare passed+diverged
doc-count to the 639 baseline, not just the divergence list. The TFM path
(`read_font_info_tfm`) is exercised by TRIP/goldens, so step 2 is dashboard-safe.
