# Scanner / `expand` SCC — landing plan (the hard core)

> Status: **analysis done (session 202), execution in progress.** This is
> the concrete plan for the last big threading frontier: the
> mutually-recursive scanner cluster headed by `expand`. Read with `01b`
> (the Phase-C recipe) — this doc is the per-cluster recon `01b` §9 calls
> for, specialized to the SCC.

## The finding (session 202)

`get_next` is threaded (`get_next_in`, session 201) and sits **below** the
cycle. Everything above it is **one strongly-connected component**:
`scan_int` → `get_x_token` → `expand` → `conv_toks`/`conditional`/
`the_toks`/`macro_call` → `scan_*` → `get_x_token` … closes the loop.

A Tarjan SCC over the (literal-stripped) call graph gives **35 functions**
mutually recursive with `expand`:

```
expand conv_toks conditional ins_the_toks macro_call          (expand.rs)
get_x_token x_token the_toks scan_toks pass_text pseudo_start
scan_int scan_dimen xetex_scan_dimen scan_glue scan_normal_glue
scan_mu_glue scan_expr scan_keyword scan_left_brace scan_register_num
scan_four_bit_int_or_18 scan_something_internal scan_char_num
scan_usv_num scan_char_class_not_ignored scan_math_fam_int
scan_font_ident find_font_dimen scan_glyph_number scan_general_text
scan_pdf_ext_toks compare_strings                              (scan.rs)
scan_and_pack_name scan_file_name scan_file_name_braced start_input (io.rs)
```

**Consequence:** none of the 35 can be threaded as a clean single-borrow
`_in` core in isolation — the moment one holds `&mut EngineState` and calls
an unconverted sibling's global wrapper it nest-panics (`01b` §4). They
must be converted **together** (one big working-tree change, validated by
the full corpus, then committed). The hot shared state
(`cur_cmd`/`cur_chr`/`cur_tok`/`cur_cs`/`cur_val`) is `mod raw`, not on
`EngineState`, so it stays raw (safe) under the borrow — this is what makes
the simultaneous conversion mechanical rather than a re-cluster.

## Prerequisite: the below-SCC `with_state` helper tail

The SCC's downward callees that are `with_state`-bearing **must already be
threaded** (or be raw/FFI/math leaves, or cold-abort deferrable) before the
SCC can hold a single borrow. Audited session 202; status:

- **Ready (threaded core exists, call under the borrow):** the `mem`
  allocators (`get_avail`/`get_node`/`free_node`/`flush_list`/`new_spec`/
  `delete_glue_ref`/`delete_token_ref`), `string_pool`
  (`make_string`/`slow_make_string`/`search_string`/`length`/`flush_str`),
  `hash` (`id_lookup`/`prim_lookup`), `save`
  (`find_sa_element`/`eq_define`), the print/error/diagnostic `_in` family,
  `input` (`back_input`/`back_error`/`begin_token_list`/…), the
  `not_*_font_error` family (`not_font_error_common_in` → `ErrorOutcome`).
- **Threaded this session (202):**
  - `str_toks_cat`/`str_toks` → `str_toks_cat_in` (`StrToksAbort`).
  - the io.rs file-name machinery `make_utf16_name`/`begin_name`/
    `more_name`/`end_name`/`make_name_string` → `_in` cores (`NameAbort`).
- **Threaded session 203 (prereq tail now drained):**
  - `prepare_mag` → `prepare_mag_in(&mut EngineState) -> ErrorOutcome`
    (save.rs) — reached from `scan_something_internal`.
  - `round_decimals(&ErrorState, k)` + `tokens_to_string_in(&mut
    EngineState)` (scan.rs; new `TokensToStringAbort`).
  - the `start_input` file-name/log path: `pack_file_name_in` /
    `pack_job_name_in` / `open_log_file_in` (io.rs) +
    `maketexstring_in(&mut StringsState)` (texmfmp.rs) — new
    `MakeTexStringAbort`/`fire_make_tex_string` and `OpenLogAbort` sentinels.
    `make_name_string` was already threaded (session 202). `start_input`'s
    other helpers (`synctex_start_input`, `u_open_in`, `input_line`, the
    print cores) were already threaded.
  - **Result:** every below-SCC `with_state`-bearing callee of the 35 SCC
    members now has an `_in` core (or is a raw/FFI leaf, or cold-abort
    deferrable). The hazard set is drained; the SCC big pass (below) is the
    next session's work. Any residual nest-panic will surface as a
    NOXDV/NOLOG corpus regression during the big pass and be fixed there.

## Cold-abort sentinels the SCC itself will need (`01b` §5)

`expand`/`conditional`/`conv_toks`/`macro_call`/the `scan_*` chain defer
their cold tails to the wrappers. Reuse the established shapes:
`ErrorOutcome` (recoverable `error()`), `InputAbort` (input-stack aborts),
`get_avail`/`get_node` `ALLOC_FAILED` → `fire_main_memory_overflow`,
`overflow("expansion depth")` / `overflow("buffer size")` /
`overflow("pool size")`. `expand` needs a new outcome enum (e.g.
`ExpandOutcome`) for its `\end`-less / fatal tails, mirroring
`GetNextOutcome`.

## Execution order for the big pass (next session)

1. Finish the prereq tail (above) — each a normal, independently-committed
   `_in` slice; corpus-validated.
2. Convert all 35 SCC members to `_in(&mut EngineState)` in the working
   tree: bodies use direct `st.<cluster>` field access + the ready cores;
   sibling calls become `*_in(st)`; cold tails ride sentinels. Use
   `thread_fn.py --apply` for the mechanical mem/eqtb accessor rewrites;
   hand-wire signatures / sibling calls / sentinels.
3. Each SCC global wrapper becomes `with_state_mut(|st| f_in(st, …))` for
   its remaining external callers (`main_control`, `math`, `shipout`, …).
4. `cargo check` + dashboard 29/2/7/4 + **full corpus** (`corpus.sh check
   all`; confirm 637/2/0, doc-count == baseline, **no NOXDV/NOLOG** — the
   file-name/`\input` paths are exactly the hidden-regression risk from the
   session-187 lesson). Commit (may be one large commit — the SCC has no
   safe smaller bisect unit; user-approved session 202).
5. Then the scanner is threaded: `get_x_token`/`scan_toks`/`the_toks`
   unlock the scanner-gated shipout hub (`ship_out`/`hlist_out`/`vlist_out`
   via `out_what`→`write_out`).
