# 02 — Module decomposition

The `READY` workstream plan for **breaking the giant translated files into
the `plan/01-architecture.md` target layout**, so that Phase-C threading can
climb into the scanner without each function being touched four times.

> Status: **READY** for execution (drafted + source-verified session 117).
> Hard rule 4 (`README.md`) is now satisfied for workstream 02: the giant
> files may be decomposed following this recipe.

Read this **with** `plan/01-architecture.md` (the destination layout) and
`01b-phase-c-recipe.md` (the threading recipe this unblocks). Where this doc
and `01b` §9 disagree on *when* to move a function vs. thread it, **this doc
wins** (newer; see Decision 1) — noted in `SESSION_LOG.md` per the README rule.

---

## 1. Why this workstream exists (the deferral it lifts)

Phase-C threading (workstream 01) is deep: the `mem` allocator core, the node
constructors, the math noad family, and the save/shipout/linebreak/synctex
leaves are all threaded (sessions 61–116). The remaining bounded leaves outside
`src/scan.rs` and `src/ext.rs` are essentially exhausted. The next real work is
the **scanner / expander / main-control** call graph — and that lives almost
entirely in one 18,689-line file, `src/scan.rs`, the faithful translation of
`xetex-xetex0.c`.

`01b` §9 deferred climbing into `scan.rs` "until 02 is READY" precisely because
threading a function there *also* wants to drop its `#[no_mangle]`, drop its
`unsafe`, and move it to its destination module — and doing all four in one
commit across an 18k-line grab-bag is unreviewable. This workstream removes the
"grab-bag" half of the problem first, mechanically and parity-trivially, so the
threading half can proceed in clean per-subsystem commits.

This is the **module-decomposition guardrail** from `00-overview.md` §6 of the
end-state target: *no file is a 10k-line grab bag.*

## 2. The destination (from `plan/01-architecture.md`)

The target `src/` is a set of modules mapping ~1:1 to the original C files,
plus a `state/` tree and a (later) `node/` tree:

```
engine.rs  state/  node/  scaled_math.rs  token.rs  scanner.rs  expand.rs
main_control.rs  pagebuilder.rs  linebreak.rs  math.rs  shipout.rs  synctex.rs
errors.rs  output.rs  format_io.rs  fonts.rs  ext.rs  pic.rs  io.rs
```

Most of these modules **already exist** (`ls src/`): `errors`, `ext`, `hash`,
`helpers`, `ini`, `input`, `io`, `linebreak`, `math`, `mem`, `mem_types`,
`output`, `pagebuilder`, `pic`, `save`, `scaled_math`, `scan`, `shipout`,
`state`, `string_pool`, `synctex`, `texmfmp`, `globals`, `consts`,
`engine_interface`. The decomposition is therefore mostly about **draining
`scan.rs` and `ini.rs` into the right existing/new modules**, not inventing a
new tree. The notable *missing* destination modules are `scanner.rs`,
`expand.rs`, `main_control.rs`, `token.rs`, and `engine.rs`/`format_io.rs`.

`globals.rs` and `mem_types.rs`/`input.rs`/`helpers.rs` are **transitional**:
`globals.rs` shrinks to nothing as Phase C deletes its accessors (its residual
`mod raw` hot-scalar set migrates into `state/` in late Phase C). They are not
"split" — they dissolve.

## 3. Current state of the giant files (verified session 117)

| File | LOC | fn defs | `#[no_mangle]` | `unsafe fn` | C origin |
|---|---:|---:|---:|---:|---|
| `src/scan.rs` | 18,689 | 222 | 151 | 192 | `xetex-xetex0.c` |
| `src/ini.rs` | 4,789 | — | — | — | `xetex-ini.c` |
| `src/globals.rs` | 4,920 | ~790 thin accessors | — | — | bridge (transitional) |
| `src/math.rs` | 4,988 | — | — | — | `xetex-math.c` (already its own module) |

`math.rs`/`linebreak.rs`/`shipout.rs` are already 1:1 with their C files and are
**not** decomposition targets (they may shed a few misplaced helpers — see §4 —
but they stay). Only `scan.rs` and `ini.rs` are grab-bags.

## 4. The decomposition map for `scan.rs` (xetex0.c)

`scan.rs`'s 222 functions cluster by TeX "part" — the C-origin ordering reflects
it. Destination module per group (anchor functions + approx. `scan.rs` line
span; spans are guides, confirm at execution time):

| Group | Destination | Anchor functions | ~span |
|---|---|---|---|
| Small int/num/glue scanners | **scanner.rs** | `scan_int`, `scan_dimen`, `xetex_scan_dimen`, `scan_glue`, `scan_eight_bit_int`, `scan_usv_num`, `scan_char_num`, `scan_register_num`, `scan_keyword`, `scan_optional_equals`, `scan_left_brace`, `scan_spec`, `scan_rule_spec`, `scan_glyph_number`, `round_decimals` | 315–1730 |
| Token-getters / expr | **scanner.rs** | `get_token`, `get_x_token`, `x_token`, `get_x_or_protected`, `get_r_token`, `scan_expr`, `scan_general_text`, `back_input` | 660–1730 |
| Token-list builders | **scanner.rs** | `scan_toks`, `the_toks`, `str_toks`, `str_toks_cat`, `ins_the_toks`, `pass_text`, `pseudo_start`, `read_toks` | 1745–2416 |
| `\…internal` / font dims | **scanner.rs** | `scan_something_internal` (huge), `scan_font_ident`, `find_font_dimen` | 2631–3857 |
| Math scanners | **scanner.rs** | `scan_math`, `set_math_char`, `scan_math_class_int`, `scan_math_fam_int`, `scan_delimiter_int`, `scan_xetex_math_char_int` | 420–8748 (scattered) |
| The tokenizer core | **scanner.rs** | `get_next`, `check_outer_validity` | 14733–15356 |
| Expansion / macros / conditionals | **expand.rs** | `expand`, `macro_call`, `conditional`, `pop_cond_node`, `conv_toks`, `ins_the_toks` | 4069–4480, 12036–15793 |
| The dispatch loop + verbs | **main_control.rs** | `main_control`, all `main_control_*` helpers, `do_lookahead`, `lig_inner_loop`, `its_all_over`, `append_glue/kern/penalty`, `make_mark`, `new_interaction`, `alter_*`, `normal_paragraph`, `indent_in_hmode`, `head_for_vmode`, `begin_insert_or_adjust`, `do_endv`, `append_discretionary`, `build_discretionary`, `shift_case`, `make_accent`, `new_font`, `do_register_command`, `unpackage`, `delete_last`, `app_space`, `new_graf`, `package`, `box_end`, `begin_box`, `scan_box`, `handle_right_brace`, `do_assignments`, `prefixed_command` (from ini.rs), `new_character`, `off_save`, `trap_zero_glue`, `fix_language` | 4808–10261, 16831–18689 |
| Alignment (`\halign`) | **main_control.rs** | `init_align`, `align_peek`, `fin_align`, `init_span/row/col`, `push/pop_alignment`, `fin_col`, `fin_row`, `get_preamble_token`, `align_error`, `no_align_error`, `omit_error` | 6049–9389 (scattered) |
| Box/node packing & list utils | **packaging.rs** (transitional → `node/`, ws03) | `hpack`, `vpackage`, `vert_break`, `vsplit`, `append_to_vlist`, `just_copy`, `just_reverse`, `copy_node_list`, `flush_node_list`, `prune_page_top`, `do_marks` | 6008–13208 (scattered) |
| Show / display / messages | **output.rs** | `show_box`, `show_node_list`, `short_display`, `short_display_n`, `show_activities`, `show_context`, `show_save_groups`, `show_whatever`, `show_info`, `print_subsidiary_data`, `issue_message`, `char_warning` | 5947–14418 (scattered) |
| Errors | **errors.rs** | `you_cant`, `report_illegal_case`, `privileged`, `cs_error`, `box_error`, `extra_right_brace`, `mu_error`, `runaway` | 315–9451 (scattered) |
| Font loading / TFM | **fonts.rs** *(new, or fold into existing font glue)* | `read_font_info`, `read_tfm_word`, `read_scaled_word`, `read_fifteen`, `tfm_getc`, `load_native_font`, `new_native_word_node`, `build_native_character`, `new_native_character`, `effective_char`, `effective_char_info` | 4772–16766 (scattered) |
| File naming / start_input / io | **io.rs** | `begin_name`, `more_name`, `end_name`, `make_name_string`, `scan_file_name`, `scan_file_name_braced`, `scan_and_pack_name`, `pack_job_name`, `pack_file_name`, `start_input`, `open_log_file`, `close_files_and_terminate` | 2416–4749, 8315–10800 (scattered) |
| Extensions / `\special` | **ext.rs** | `do_extension`, `new_write_whatsit`, `append_src_special`, `insert_src_special`, `tt_insert_special` | 5515–11053 (scattered) |
| Locale line-break support | **linebreak.rs** | `do_locale_linebreaks`, `char_pw_in` | 10368, 16766 |

`token.rs` is small: the token/command **constants** plus the hot
`cur_cmd`/`cur_chr`/`cur_tok`/`cur_cs`/`cur_val` accessors (when those migrate
off `mod raw` — see Decision 4). It can be created last.

## 5. The decomposition map for `ini.rs` (xetex-ini.c)

| Group | Destination | Anchor functions |
|---|---|---|
| Init / run / cleanup | **engine.rs** *(new)* | `tt_run_engine`, `tt_cleanup`, `final_cleanup`, `initialize_more_variables`, `initialize_more_initex_variables`, `initialize_primitives`, `primitive`, `sort_avail`, `get_strings_started` |
| Format dump / load | **format_io.rs** *(new)* | `store_fmt_file`, `load_fmt_file`, `bad_format_file`, `pack_buffered_name`, all `dump_*` / `undump_*` |
| Hyphenation trie build | **hyphenation.rs** *(new; extends 01-arch layout — flag)* OR fold into `linebreak.rs` | `init_trie`, `new_trie_op`, `trie_op_hash`, `trie_node`, `compress_trie`, `first_fit`, `trie_pack`, `trie_fix`, `new_patterns`, `new_hyph_exceptions` |
| Assignment verbs | **main_control.rs** | `prefixed_command`, `do_assignments`, `intpar` |
| io init | **io.rs** | `init_io` |

## 6. Key decisions (baked into this plan)

**Decision 1 — Mechanically pre-split *first*, then thread in place.**
`00-overview.md` §169–175 explicitly sanctions a *purely mechanical* pre-split
(move function groups into submodules, **no signature/behavior change**) and
says it "could precede 01." Adopt it: do the mechanical move **before** the
scanner threading, not interleaved with it. Rationale: a mechanical move is
parity-trivial (visibility + `mod`/`use` only; `#[no_mangle]` symbols are
global, so C linkage is unaffected; cross-module calls just need `pub(crate)`),
whereas threading is semantic and per-function. Separating them keeps each
commit small and bisectable — the opposite of the unreviewable "4-in-1 across
18k lines" that `01b` §9 feared. **This supersedes `01b` §9's "thread + move
together" suggestion** for the giant-file case; with the move done first,
threading then happens *in the destination module*, one subsystem per commit,
exactly like every prior Phase-C slice.

**Decision 2 — Move order: leaves out first, core last.** Drain the
*already-have-a-home* groups out of `scan.rs` before carving the new
scanner/expand/main_control trio: `output.rs` (show/display), `errors.rs`
(`you_cant`/`runaway`/…), `io.rs` (file naming/`start_input`), `ext.rs`
(`do_extension`/`\special`), `fonts.rs` (TFM/native loading),
`linebreak.rs` (`do_locale_linebreaks`/`char_pw_in`). Each is a small,
self-contained relocation. Only then split the residual scanner / expander /
main-control / alignment / packaging into their modules. `ini.rs` drains in
parallel (`format_io.rs`, `engine.rs`, hyphenation) — independent of `scan.rs`.

**Decision 3 — `packaging.rs` is a transitional home.** `hpack`/`vpackage`/
`vsplit`/`vert_break`/`copy_node_list`/`flush_node_list`/`just_copy`/
`just_reverse`/`prune_page_top`/`do_marks` are node-layer code destined for the
`node/` tree, but `node/` depends on the typed-accessor design (workstream 03,
`plan/03-memory-and-nodes.md`), which is **not built**. Park them in a flat
`packaging.rs` now; they fold into `node/` when ws03 lands. Do **not** build
`node/` in this workstream.

**Decision 4 — Hot tokenizer scalars stay raw through the split.**
`cur_cmd`/`cur_chr`/`cur_tok`/`cur_cs`/`cur_val`, `cur_input`, `cur_list`,
`line`, `cur_ptr` remain raw `mod raw` globals during the decomposition *and*
the initial scanner threading (per `01b` §4 they are `unsafe { raw::… }`, safe
under a live borrow). Migrating them into `EngineState` is a **separate, later,
benchmarked slice** (`bench_phase_c.py` is the harness) — not part of 02.
`token.rs` gets their accessors only once they migrate.

**Decision 5 — `state.rs` → `state/` tree is an independent sub-task.** Splitting
the single `state.rs` into `state/{mod,mem,eqtb,save_stack,…}.rs` is mechanical
and parity-trivial but orthogonal to the `scan.rs`/`ini.rs` drain. Lowest
priority; do it any time as filler. Not on the critical path.

**Decision 6 — `globals.rs` is not split; it dissolves.** Its accessors are
deleted per Phase-C slice (recipe step 6). 02 does not relocate it. When Phase C
finishes, the residual `mod raw` set is all that remains, and it migrates into
`state/`.

## 7. Execution recipe (per mechanical-move slice)

Run from the crate root. One function group (one destination module) per commit.

1. **Pick a group** from §4/§5 (Decision 2 order: leaves first).
2. **Create/locate the destination module.** New modules (`scanner.rs`,
   `expand.rs`, `main_control.rs`, `packaging.rs`, `engine.rs`,
   `format_io.rs`, `hyphenation.rs`) need a `mod` line in `lib.rs`.
3. **Move the function bodies verbatim — with the script, not by hand.** Use
   `plan/idiom/move_fns.py`, which extracts each top-level `fn` (with its
   leading `// translated from …` comment + `#[…]` attributes, through the
   column-0 closing brace) and relocates it unchanged. Editing function bodies
   across with the manual edit tool is pure token waste and risks transcription
   error — the move changes **no bytes inside any function**, so it is exactly a
   scripted operation.
   ```bash
   # dry run first — eyeball the line ranges:
   python3 plan/idiom/move_fns.py --src src/scan.rs --dst src/errors.rs \
       --fns you_cant,report_illegal_case,privileged,cs_error,box_error,mu_error,runaway,extra_right_brace
   # then apply:
   python3 plan/idiom/move_fns.py --src ... --dst ... --fns ... --apply
   ```
   **Do not change signatures, `#[no_mangle]`, `unsafe`, or bodies** — this
   commit is behavior-preserving relocation only.
4. **Fix visibility + imports.** Functions called cross-module become
   `pub(crate)`. Add `use crate::scanner::…` (etc.) at call sites; move the
   relevant `use crate::globals::{…}` / `use crate::consts::{…}` imports into
   the destination module. Module-private consts used by a moved fn move with it
   (or become `pub(crate)` in a shared home).
5. **`cargo check`** — resolve visibility/import errors (the bulk of the work).
6. **Full dashboard. Never regress** (hard rule 1):
   ```bash
   cargo test -p tectonic --test tex-outputs --test trip \
     --test engine_rust --test latex_goldens     # 29 / 2 / 7 / 4
   ```
7. **Update docs + commit** (`SESSION_LOG.md`, `STATUS.md`), prefix
   `engine_xetex:`. Note the LOC drop in `scan.rs`/`ini.rs`.

**Parity argument:** a pure relocation changes no emitted byte. `#[no_mangle]`
symbols are global regardless of defining module, so C linkage is unchanged;
the only failures are compile-time visibility/import errors, caught by step 5.
This is the safest possible class of large change — which is why it goes first.

## 8. Sequencing & first slice

**First slice:** drain `show_*` / display / messaging into `output.rs` (the
largest clean leaf group, ~1.5k LOC, no scanner entanglement) — *or*, even
safer, drain the `errors.rs` group (`you_cant`/`report_illegal_case`/
`privileged`/`cs_error`/`box_error`/`mu_error`/`runaway`, small + self-contained).
Either proves the recipe before the bigger scanner/main_control carve.

**Then:** `io.rs` (file naming/`start_input`), `ext.rs`, `fonts.rs` leaves →
then `ini.rs` drains (`format_io.rs`, `engine.rs`, hyphenation) → finally the
core carve: `scanner.rs`, `expand.rs`, `main_control.rs`, `packaging.rs`.

**After 02 (or once the scanner module exists), Phase-C threading resumes**
inside the now-navigable `scanner.rs`/`expand.rs`/`main_control.rs`, one
subsystem per commit per `01b` — `ext.rs` and the `eq_define`/`prefixed_command`
families then thread without bridge-churn worries, because their callers are now
in threaded modules.

**Coordination:** 02 (this) and 01 (Phase C) interleave from here. Do **not**
thread *and* move the same function in one commit (Decision 1). Workstream 03
(de-unsafe / `node/` typed accessors) follows; 04 (clippy) and 05 (errors) are
consequences.

## 9. READY checklist (satisfied session 117)

- [x] Destination layout pinned to `plan/01-architecture.md`.
- [x] Current giant-file sizes / counts verified by direct grep (§3).
- [x] `scan.rs` (222 fns) and `ini.rs` inventoried and group-mapped (§4/§5).
- [x] Move-vs-thread ordering decided (Decision 1) and the `01b` §9 conflict
      noted.
- [x] Transitional homes (`packaging.rs`) and out-of-scope work (`node/`,
      hot-scalar migration, `state/` split) explicitly fenced.
- [x] Mechanical-move recipe + parity argument written (§7).
- [x] First slice chosen (§8).

Open items for the first execution session to confirm (not blockers): exact
line spans (the §4 spans are guides), and whether hyphenation-trie code lands in
a new `hyphenation.rs` or folds into `linebreak.rs` (recommend the former; it is
a coherent ~900-LOC unit and keeps `linebreak.rs` at its C-file scope).
