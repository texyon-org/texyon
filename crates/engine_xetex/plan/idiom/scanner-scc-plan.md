# Scanner / `expand` SCC — landing plan (the hard core)

> Status: **DONE (session 204).** The big pass landed: all 37 SCC members
> are threaded into `_in(&mut EngineState) -> Result<T, SccAbort>` cores,
> each global is a thin `with_state_mut` wrapper, and the corpus is green
> (no NOXDV/NOLOG). See `SESSION_LOG.md` session 204. The manifest below is
> retained as the executed design of record. This was the last big
> threading frontier: the mutually-recursive scanner cluster headed by
> `expand`. Read with `01b` (the Phase-C recipe).

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

1. **DONE (session 203).** The prereq tail is fully threaded and
   corpus-validated; see §"Threaded session 203" above. Start at step 2.
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

## Execution manifest (session 203 recon — make the big pass mechanical)

Session 203 prototyped the conversion (`SccAbort` + the 9 trivial leaf/
delegator cores) then **reverted the source WIP**: the SCC is one
all-or-nothing compile+validate unit (~4000 lines incl. the
`expand`/`macro_call`/`conditional`/`scan_something_internal`/`scan_dimen`/
`scan_expr` monsters), genuinely a multi-turn job, and a half-done wired
push leaves `todo!()` stubs in live wrappers — uncommittable. Better to
land the concrete design here so the real pass is mechanical, and execute
it with **incremental compilation** (below) rather than blind.

### a) The `SccAbort` sentinel (drop into scan.rs, near `GetNextOutcome`)

```rust
pub(crate) enum SccAbort {
    Error(crate::errors::ErrorOutcome),     // recoverable error gone terminal
    Input(crate::input::InputAbort),        // input-stack / tokenizer cold abort
    NoLegalEnd { open_log: bool },          // \end-less job
}
impl SccAbort {
    pub(crate) unsafe fn fire(self) -> ! {  // call AFTER the borrow drops
        match self {
            SccAbort::Error(oc) => { oc.finish(); unreachable!() } // finish() returns only for Continue (never stored)
            SccAbort::Input(a) => a.fire(),
            SccAbort::NoLegalEnd { open_log } => {
                if open_log { crate::io::open_log_file(); }
                crate::errors::fatal_error(c"*** (job aborted, no legal \\end found)".as_ptr());
                unreachable!("fatal_error returned")  // fatal_error is typed () in Stage 1
            }
        }
    }
}
// And `impl From<GetNextOutcome> for Result<(), SccAbort>` so `get_x_token_in`
// can write `Result::from(get_next_in(st))?`. (`fatal_error` is `()` not `!`;
// `fire()` must append `unreachable!()` to be `-> !` — needed so i32/bool
// wrappers can write `Err(e) => e.fire()`.)
```

Every SCC `_in` returns `Result<T, SccAbort>` (T = `()` for most, `i32` for
`the_toks`/`scan_toks`, `bool` for `scan_keyword`). Sibling calls propagate
with `?`. Recoverable error sites: `let o = error_in(st)/int_error_in(st,n);
if !matches!(o, ErrorOutcome::Continue) { return Err(SccAbort::Error(o)); }`
then continue (mirrors `prepare_mag_in`, session 203).

### b) ⚠ The nest-panic audit is the real hazard (not the accessor rename)

`thread_fn.py` renames regular `mem_*`/`eqtb_*` accessors, but the bodies
take `st: &mut EngineState` (these fns touch *many* clusters — no narrow
borrow), so accessors become **`st.mem.b32s1(p)` / `st.eqtb.b32s1(p)` /
`st.scanner.<field>`**, not `mem.b32s1`. **Every** accessor call must be
classified, because a missed `with_state`-bearing call nest-panics at
runtime, invisible to the compiler (caught only by the corpus). Rule:

- **Raw-safe, keep the accessor call as-is** (the deferred `mod raw` set,
  `globals.rs:52`): `cur_val`/`cur_tok`/`cur_cmd`/`cur_chr`/`cur_cs`,
  `line`, `first`, `cur_input` (+ its `set_cur_input_*`), `cur_ptr`,
  `temp_ptr`, `cur_list`, `expand_depth`, `save_size`, `buf_size`,
  `nest_size`, `stack_size`, `param_size`, `max_in_open`, the `main_*` /
  lig-builder scratch. These read `unsafe { raw::… }`, **not** `with_state`.
- **Everything else is `with_state` → convert** to direct `st.<cluster>.field`
  or a threaded `_in`/substate helper. Landmines found session 203:
  `max_reg_num()`→`st.scanner.max_reg_num`, `max_reg_help_line()`→
  `st.scanner.max_reg_help_line`, `set_help_ptr`/`set_help_line(k,_)`→
  `st.errors.help_ptr`/`st.errors.help_line[k]`, `selector()`→
  `st.shipout.selector`, `align_state()`→`st.scanner.align_state`, etc.

Worked leaf-scanner exemplar (the 9 trivial members all follow this):

```rust
pub(crate) fn scan_char_num_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > BIGGEST_CHAR {            // cur_val: raw-safe
        let errmsg = error_here_with_diagnostic_in(st, c"Bad character code".as_ptr());
        unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val()) }; // FFI: narrow unsafe
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;                              // was set_help_ptr (with_state)
        st.errors.help_line[1] = c"A character number must be between 0 and 65535.".as_ptr();
        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
        let o = int_error_in(st, cur_val());
        if !matches!(o, ErrorOutcome::Continue) { return Err(SccAbort::Error(o)); }
        set_cur_val(0);                                      // set_cur_val: raw-safe
    }
    Ok(())
}
```

Make `_in` cores **safe fns with narrow `unsafe {}`** around FFI
(`ttstub_diag_printf`, libc) / raw-ptr ops (precedent: `get_next_in`); this
keeps the `unsafe fn` metric from ballooning vs. making the cores `unsafe fn`.

### c) The 9 trivial members (mechanical, ~exemplar above)

`scan_char_num`/`scan_usv_num`/`scan_char_class_not_ignored`/
`scan_math_fam_int`/`scan_register_num`/`scan_four_bit_int_or_18` (leaf
range-check on `scan_int`); `scan_normal_glue`=`scan_glue_in(st,GLUE_VAL)`,
`scan_mu_glue`=`scan_glue_in(st,MU_VAL_SCAN)`, `scan_pdf_ext_toks`=
`scan_toks_in(st,false,true)?`. **NB:** the look-alike `scan_eight_bit_int`/
`scan_math_class_int`/`scan_four_bit_int`/`scan_fifteen_bit_int`/
`scan_delimiter_int`/`scan_xetex_math_char_int` are **NOT** SCC members
(scan_int doesn't call back into them) — leave them raw, calling the
`scan_int()` wrapper; they don't hold a borrow so they don't nest-panic.

### d) Incremental-compilation strategy (do NOT translate 35 blind)

1. Add `SccAbort`. Add **all 35 `_in` stubs** (`fn f_in(st,…) -> Result<T,
   SccAbort> { todo!() }`, correct signatures) — compiles.
2. Wire **all 35 wrappers** to delegate: `()`→`if let Err(e) =
   with_state_mut(|st| f_in(st,…)) { e.fire() }`; `i32`/`bool`→`match … {
   Ok(v)=>v, Err(e)=>e.fire() }`. Compiles. (Tests now panic on `todo!()`
   — do **not** run them until step 3 finishes.)
3. Fill each `_in` body (translate from the global version per §b),
   `cargo check` after each — incremental error-catching, still "one big
   push" since validation is deferred to the end.
4. Only when **all 35** bodies are filled: dashboard + full corpus.
5. Delete now-dead `globals.rs` accessors the conversion orphaned
   (`max_reg_num`/`max_reg_help_line`/`set_help_*`/… — grep each).

This keeps the build green per function and reaches the same one-commit end
state. The 35 wrappers stay (external non-SCC callers); they are the new
`with_state_mut` entry points, so the SCC holds exactly one borrow per entry.

### e) Member sizes (rough; budget the session accordingly)

Trivial (9, §c). Small/medium (~20): `x_token`, `get_x_token`,
`scan_left_brace`, `scan_keyword`, `scan_and_pack_name`, `scan_file_name`,
`scan_file_name_braced`, `compare_strings`, `scan_glyph_number`,
`scan_font_ident`, `find_font_dimen`, `scan_general_text`, `pass_text`,
`pseudo_start`, `conv_toks`, `ins_the_toks`, `the_toks`. Large (~6, the
real work): `scan_int`, `scan_dimen`/`xetex_scan_dimen`, `scan_glue`,
`scan_expr`, `scan_toks`, `conditional`, `expand`, `macro_call`,
`scan_something_internal` (~900 lines, the biggest), `start_input`.
