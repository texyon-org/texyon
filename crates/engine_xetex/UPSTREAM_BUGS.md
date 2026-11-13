# XeTeX bugs found while porting

Bugs in upstream `xetex.web` that the port ran into, with what TeXyoN does about each. Line
numbers are in `reference_sources/stage2/exports/xetexdir/xetex.web` (XeTeX 0.999994) unless an
entry names a later TeX Live version.

## 1. `\XeTeXinterchartoks 4096` indexes past the sparse-array tree

- **Where:** `scan_char_class_not_ignored` (line 10249).
- **Bug:** it rejects only `cur_val > char_class_limit`, so 4096 passes, although its help text
  says classes run from 0 to 4095. The interchartoks index `4096 * char_class_limit + c` is then
  `2^24` or more. `find_sa_element` takes `hex_dig1(n) = 64`, and `get_sa_ptr` reads and writes
  `q + 33`, one word past the 33-word index node, which belongs to some other node.
- **Repro:** `\XeTeXinterchartokenstate=1 \XeTeXinterchartoks 4096 0 {x}`. Upstream carries on
  silently with a corrupted neighbour. Probe `mainloop-interchartoks_tfm`.
- **Fix:** `if (cur_val<0)or(cur_val>=char_class_limit)`.
- **TeXyoN:** rejects 4096 with "Bad character class" (`scan/numbers.rs`). Listed in
  `tests/upstream/known.tsv`, test `interchartoks_classes_stop_below_the_ignored_class`.

## 2. `do_marks` visits 16 of the 64 slots of each index node

- **Where:** `do_marks` (line 33766), `for i:=0 to 15 do`.
- **Bug:** XeTeX widened e-TeX's sparse arrays from 16 to 64 children per index node (the
  comment still says "a four bit index"), but `do_marks` kept e-TeX's 16. Mark classes with any
  6-bit digit of 16 or more are never visited, so at a page break or `\vsplit` their
  `\topmarks`/`\firstmarks`/`\botmarks` do not move on, and `\vsplit` never clears their split
  marks.
- **Repro:** `\marks20{B}` on page 1, then `\topmarks20` on page 2 is still empty. Confirmed
  against TeX Live 2025 xetex.
- **Fix:** `for i:=0 to 63 do`.
- **TeXyoN:** visits all 64 (fixed 2026-11-12), test `every_mark_class_carries_over`.

## 3. Interword space shaping assumes the list tail is a native word (needs a repro)

- **Where:** the `\XeTeXinterwordspaceshaping` block around line 24545.
- **Bug:** its comment says the scan "is guaranteed to end safely because it'll eventually hit
  `tail`, which is a `native_word` node", and the code then reads `native_length(tail)` and
  `get_native_char(tail, t)` without checking the type. The port found a list where the tail was
  not a native word, so these reads take arbitrary words of another node.
- **TeXyoN:** skips the kern when the tail is not a native word (`shape_interword_space`).
- **Before reporting:** build a minimal document that reaches this with a non-native tail.

## 4. `\ifcsname` with one character above U+FFFF reads past `null_cs`

- **Where:** TeX Live trunk `xetex.web` (0.999996 on), `if_cs_code` (line 32813). Since the
  2024-05-18 change, `null_cs=single_base+too_big_char`, and get_next and `\csname` look up a
  single character above U+FFFF in the hash.
- **Bug:** `\ifcsname` was not updated: `else cur_cs:=single_base+buffer[first]` still takes the
  `single_base` slot for such a character, which lies past `null_cs` in the hash or later
  regions, so the test reads an unrelated `eqtb` entry.
- **Repro:** `\ifcsname 😀\endcsname yes\else no\fi` with `\😀` undefined says yes with TeX Live
  2025 xetex. Probe `etex-ifcsname_astral_undefined`.
- **Fix:** test `(m>first+1)or(buffer[first]>@"FFFF)` before `m=first`, as in `\csname`.
- **TeXyoN:** looks the name up in the hash (`expand.rs`). Listed in `tests/upstream/known.tsv`.

## 5. `\partokenname` after a prefix or `\accent` is a confusion

- **Where:** TeX Live `partoken-102.ch`/`partoken.ch` (TL 2022 on). `partoken_name=103` becomes
  `max_command`, above `max_non_prefixed_command`.
- **Bug:** `prefixed_command` and `do_assignments` treat every command above
  `max_non_prefixed_command` as an assignment, but `prefixed_command`'s case has no
  `partoken_name` branch. So `\global\partokenname\foo` (or `\long`, `\outer`, `\protected`)
  and `\accent65 \partokenname\foo`, with no prefix, end in `confusion("prefix")`.
  `do_assignments` also runs after a display alignment.
- **Repro:** either line gives "This can't happen (prefix)" with TeX Live 2025 xetex. Probe
  `errors-partokenname_prefix`. pdfTeX shares the change file.
- **Fix:** test `cur_cmd>max_non_prefixed_command` and `cur_cmd<partoken_name` in both places, or
  number `partoken_name` below `max_non_prefixed_command`.
- **TeXyoN:** the first fix (`Cmd::MAX_PREFIXED_COMMAND`). A prefix gets "You can't use a prefix
  with", and `do_assignments` stops at `\partokenname`, which then runs as usual. Test
  `partokenname_takes_no_prefix_and_ends_assignments`, listed in `tests/upstream/known.tsv`.

## 6. A file name scanned inside another command replaces the outer name

- **Where:** `new_font` (`@<Scan the font size specification@>`) and `load_picture`, with any
  primitive that calls `scan_file_name` during expansion: `\XeTeXpdfpagecount`,
  `\XeTeXglyphindex`, `\XeTeXfindfeaturebyname` and the other AAT/Graphite name lookups.
- **Bug:** the scanned name lives in the globals `cur_name`, `cur_area`, `cur_ext` (and
  `name_of_file`). `new_font` scans the font name, then scans the size, which may expand one of
  those primitives. Its own `scan_file_name` overwrites the globals, and `new_font` then looks up
  and loads that name. `name_in_progress` only blocks `\input`. `load_picture` reads
  `name_of_file` after scanning `page`, so `\XeTeXpdffile "b.pdf" page \XeTeXpdfpagecount "c.pdf"`
  includes `c.pdf`.
- **Repro:** `\font\x=cmr10 at \XeTeXpdfpagecount "cmr5" pt \message{\fontname\x}` prints
  `cmr5 at 10.0pt` with TeX Live 2025 xetex. Probe `xetex-nested_name_in_font_size`.
- **Fix:** save the scanned name in locals before scanning further, as `new_font` already does
  for `u` and `t`.
- **TeXyoN:** each scan returns its own name, so the font is `cmr10` and the picture `b.pdf`.
  Listed in `tests/upstream/known.tsv`, test `a_name_scanned_inside_a_font_size_keeps_the_font_name`.

## 7. `\tracingstacklevels` traces the first input file as the log file

- **Where:** `start_input`, TeX Live's `\tracingstacklevels` change (`print("INPUT ")`).
- **Bug:** for the file that sets `job_name`, `open_log_file` runs first. It packs `<job>.log` and
  rescans it through `cur_name`/`cur_ext`, so the trace after it prints `~INPUT <job>.log`.
- **Repro:** dump a format with `\tracingstacklevels=1`, then run any file on it. Not reachable in
  the upstream harness, whose formats leave the parameter at 0.
- **Fix:** print the trace before `open_log_file`, or save `cur_name`/`cur_ext` around it.
- **TeXyoN:** prints the input's own name, test `the_first_input_is_traced_by_its_own_name`.

## 8. An unquoted native font is loaded under its TFM lookup name

- **Where:** `read_font_info` and `check_for_tfm_font_mapping` (XeTeX_ext.c).
- **Bug:** an unquoted name is tried as a TFM font first. For that lookup `name_of_file` is packed
  without the extension, and `check_for_tfm_font_mapping` writes a NUL over `:mapping=`. When no
  TFM exists, the native fallback reads that buffer. Everything after the last `.` is lost, so
  `[lmroman12-regular.otf]:mapping=tex-text` loads `[lmroman12-regular` with no mapping and
  `Font:letterspace=50.5` gets letterspace 50. A `:mapping=` is dropped too, and for a `[file]`
  name `load_native_font` copies the buffer past the NUL into the font name.
- **Repro:** `\font\x=[lmnoext]:mapping=tex-text` (an extensionless copy of an OpenType font) gives
  `\fontname` `"[lmnoext]^^@mapping=tex-text"` with TeX Live 2025 xetex, and `a--b` keeps its
  hyphens. Quoted, the same name gets the mapping. Probe `xetex-unquoted_native_font_name`.
- **Fix:** pass the name as written to the native fallback and keep the TFM form for the TFM
  lookup only.
- **TeXyoN:** that fix. Unquoted and quoted names load the same native font when no TFM exists.
  One cost: unquoted `DejaVuSans.ttf` no longer finds the installed `DejaVuSans`, as quoted
  `"DejaVuSans.ttf"` already did not. Listed in `tests/upstream/known.tsv`, tests
  `an_unquoted_native_font_keeps_its_name_as_written` and `an_unquoted_native_font_loads_its_mapping`.

## 9. A wrapped `\prevgraf` reuses the previous paragraph's hanging indent

- **Where:** `line_break`, `@<Get ready to start line breaking@>`, and its readers in `try_break`
  and `post_line_break`.
- **Bug:** `first_width` and `first_indent` are set only when `\hangindent` is nonzero. They are
  read for lines numbered at most `last_special_line`, which is 0 without a hanging indent, so
  normally never. A `\prevgraf` near 2^31 wraps the line numbers negative, and then every line
  takes `first_width` and `first_indent` as the last hanging paragraph left them.
- **Repro:** after `\hangindent=20pt \hangafter=-5` on one paragraph, a paragraph with
  `\noindent\prevgraf=2147483647` and `\hsize=40pt` is set 20pt wide and shifted 20pt by TeX Live
  2025 xetex. Probe `par-wrapped_prevgraf_stale_hang`.
- **Fix:** set `first_width:=hsize` and `first_indent:=0` when `hang_indent=0`.
- **TeXyoN:** that fix, so the paragraph keeps the full `\hsize`. Listed in
  `tests/upstream/known.tsv`, test `a_wrapped_prevgraf_without_hanging_indent_keeps_the_full_measure`.

## 10. A dimension scanned during another dimension's units clears its overflow

- **Where:** `scan_dimen`, which starts with `arith_error:=false` and ends by testing it.
- **Bug:** `arith_error` is global and `scan_dimen` is not reentrant. After `true` has scaled the
  number, the unit keywords are scanned with expansion. An `\ifdim` expanded there runs
  `scan_dimen` again, which clears the flag the outer call has already set. `scan_expr` saves
  and restores the flag for this reason, `scan_dimen` does not.
- **Repro:** with `\mag=1`, `\dimen0=3000000 true sp` gives "Dimension too large", and
  `\dimen0=3000000 true\ifdim0pt=0pt \fi sp` gives no error and `1.39697pt` in TeX Live 2025
  xetex. Probe `limits-nested_dimen_overflow`.
- **Fix:** save `arith_error` on entry to `scan_dimen` and restore it on exit, as `scan_expr` does.
- **TeXyoN:** each dimension keeps its own overflow, so both report "Dimension too large". Listed in
  `tests/upstream/known.tsv`, test `a_dimension_scanned_inside_another_keeps_the_outer_overflow`.

## 11. A register reached through `\primitive` takes a decimal fraction

- **Where:** `scan_int`, `@<Reset |cur_tok| for unexpandable primitives, goto restart@>`, and
  `scan_dimen`'s test `(radix=10) and (cur_tok=point_token)`.
- **Bug:** `scan_int` sets `radix:=0` once, before `restart`. When `\primitive` leads to an
  internal quantity, `scan_something_internal` scans the register number with a nested
  `scan_int`, which leaves its own `radix` behind. `scan_dimen` then sees `radix=10` and reads a
  following `.5` as the fraction of the register's value.
- **Repro:** with `\count5=3`, `\dimen0=\primitive\count5.5pt` gives `3.5pt` in TeX Live 2025
  xetex. `\dimen0=\count5.5pt` and `\dimen0=\primitive\count"5.5pt` give "Illegal unit of
  measure" and `3.0pt`. Probe `prim-primitive_register_fraction`.
- **Fix:** set `radix:=0` after `scan_something_internal` in `scan_int`.
- **TeXyoN:** that fix, so all three give the error. Listed in `tests/upstream/known.tsv`, test
  `a_register_value_takes_no_decimal_fraction`.

## 12. A sparse register lookup after a native word drops the next token

- **Where:** the end of `collect_native` in the main loop, `if cur_ptr<>null then goto big_switch
  else goto reswitch`, with `check_for_inter_char_toks` and `check_for_post_char_toks`.
- **Bug:** `cur_ptr` is meant to say that inter-character tokens were inserted and the current
  token was backed up. It is the shared result of `find_sa_element`, so it is also non-null when
  the expansion that ended the word looked up any register above 255, and when a class pair has
  a record whose token list is empty. In both cases nothing was backed up, and `goto big_switch`
  fetches a new token over the current one.
- **Repro:** in a native font, `\count300=5 abc\ifnum\count300=5 \fi\message{[B]}` typesets
  `[B]` in TeX Live 2025 xetex. With `\count200` the message is written. Probe
  `native-sparse_lookup_drops_token`.
- **Fix:** go to `big_switch` only when the token list was actually inserted, with a local flag.
- **TeXyoN:** that fix. Listed in `tests/upstream/known.tsv`, test
  `a_sparse_register_lookup_after_a_native_word_drops_no_token`.

## 13. A vertical OpenType assembly takes its width from a glue node

- **Where:** `stack_glue_into_box`, line 16993, `height(b):=height(p); width(b):=width(p)` on the new glue node `p`.
- **Bug:** `height` and `width` are box fields. On a glue node `width(p)` is the leader pointer, which is
  null (`min_halfword`), and `height(p)` reads the word after the node. The height is harmless, since the
  next glyph's `stack_glyph_into_box` overwrites it. The width is not: it resets the running maximum, so
  the assembled box gets the widest part since the last connector glue instead of the widest part overall.
- **Repro:** none yet. Needs a vertical assembly whose parts differ in width, such as a tall brace in an
  OpenType math font.
- **Fix:** leave `width(b)` alone, so it stays the maximum over all parts.
- **TeXyoN:** follows TeX Live (`width(b)` set to null). Not reported yet.

## 14. `\showgroups` prints the `\vadjust pre` flag as the insert number

- **Where:** `show_save_groups`, line 31525, `if saved(-2)=255 ... print_int(saved(-2))`.
- **Bug:** e-TeX kept only the insert number below an insert group, so `saved(-2)` (above the line
  word) was that number. XeTeX's `\vadjust pre` (line 25742) pushes a second word, the pre flag, on top
  of it, and `show_save_groups` was not moved down. It reads the flag: `\vadjust` and `\insert100` show
  as `\insert0`, `\vadjust pre` as `\insert1`.
- **Repro:** `\insert100{\showgroups}` prints `(\insert0{)`. Confirmed against TeX Live 2025 xetex.
- **Fix:** read the number from `saved(-3)` and the flag from `saved(-2)`.
- **TeXyoN:** prints `\insert100{`, `\vadjust{` and `\vadjust pre{` (`output/show.rs`). Listed in
  `tests/upstream/known.tsv`, test `showgroups_names_inserts_and_vadjusts`. Not reported yet.

## 15. Inter-character tokens after a TFM ligature lose the ligature

- **Where:** `main_loop_lookahead+1`, line 24713, `check_for_inter_char_toks(big_switch)`.
- **Bug:** when the lookahead character gets inter-character tokens, the main loop jumps to
  `big_switch` without `wrapup`. A ligature that was pending is never packed, so its characters stay
  as plain char nodes, and `ligature_present` stays true. The next run of the main loop packs its
  first character as a one-character ligature. The right boundary of the word is skipped as well.
- **Repro:** `\XeTeXinterchartokenstate=1 \XeTeXcharclass`a=1 \XeTeXinterchartoks 0 1={\relax}`,
  then `\hbox{fia}` in `cmr10` gives `f`, `i`, `a (ligature a)`. Confirmed against TeX Live 2025 xetex.
- **Fix:** end the word there as for any non-character token (`cur_r:=bchar; lig_stack:=null; goto
  main_lig_loop`), and go to `big_switch` instead of `reswitch` once the word is wrapped up.
- **TeXyoN:** ends the word, so `fia` gives `fi (ligature)`, `a`, and the boundary ligatures and kerns
  of the font apply (`main_control/main_loop.rs`). The ligature flags are per run. Listed in
  `tests/upstream/known.tsv`, probe `mainloop-interchar_after_ligature`, test
  `inter_char_tokens_end_the_word_before_them`. Not reported yet.

## 16. `\noboundary` before a native-font character cancels a later left boundary

- **Where:** `main_loop`, line 24613. `cancel_boundary` is set at line 24190 and cleared only here,
  after the `collect_native` branch has been taken for a native font.
- **Bug:** `\noboundary` before a character of an AAT or OpenType font leaves `cancel_boundary` set.
  It then cancels the left boundary of the next word in a TFM font, however far away.
- **Repro:** with a TFM font `\xa` that has a left boundary ligature for `k` and a native font
  `\nat`, `\hbox{\nat \noboundary a\xa klm}` leaves `k` plain. Confirmed against TeX Live 2025 xetex.
- **Fix:** clear `cancel_boundary` before the `collect_native` branch.
- **TeXyoN:** `\noboundary` applies to the character right after it and nothing else
  (`main_control.rs`). Listed in `tests/upstream/known.tsv`, probe `mainloop-noboundary_native`. Not
  reported yet.

## 17. Hyphenation marks the left boundary with a real character code

- **Where:** `hyphenate`, lines 21729, 21741 and 22113. They set `hu[0]` or `hu[l]` to
  `max_hyph_char` where `tex.web` sets 256, its `non_char`. `reconstitute` (lines 21915-21931)
  still tests `cur_l=non_char`, and XeTeX's `non_char` is `too_big_char` (line 12853).
- **Bug:** when the font has a left boundary program, the marker no longer reads as the boundary.
  `reconstitute` takes it for character `max_hyph_char`: it appends a character node with that code
  and reads `char_info` for it, past `font_ec`, so whatever `font_info` word lies there acts as its
  lig/kern start. The boundary ligatures and kerns of the font are lost.
- **Repro:** a TFM font with `(LABEL BOUNDARYCHAR) (KRN C a R 0.2)`, `\hyphenation{a-a}` and a
  narrow `\hsize`. The line after the hyphen starts with a bogus character instead of the 0.2 kern.
  Confirmed against TeX Live 2025 xetex in probe `hyph-bigtable_hyph` and the `hyph-recon_*` probes.
  Knuth's `tex` gives the boxes TeXyoN gives on all of them.
- **Fix:** set the marker to `non_char`, as `tex.web` does with 256.
- **TeXyoN:** writes `non_char` (`linebreak/hyphenate.rs`), so the boundary program runs and no
  character is added. Listed in `tests/upstream/known.tsv` (18 hyphenation probes), test
  `left_boundary_after_a_hyphen_is_no_character`. Not reported yet.

## 18. The OpenType script list always comes from GSUB

- **Where:** `getLargerScriptListTable` in `XeTeXLayoutInterface.cpp` (TeX Live 2025), behind
  `\XeTeXOTcountscripts` and `\XeTeXOTscripttag`.
- **Bug:** it means to return the longer of the GSUB and GPOS script lists, but the GPOS call
  passes `HB_OT_TAG_GSUB` again. HarfBuzz caps the second fetch at the GSUB count, so the result is
  always GSUB's list. A font with scripts only in GPOS reports none.
- **Repro:** FandolSong-Regular has no GSUB, and its GPOS lists `DFLT`, `hani`, `kana` and `latn`.
  Upstream counts 0 scripts, so fontspec warns "Script 'CJK' not explicitly supported" and drops
  the script from font requests. Suite case `docs/cjk`.
- **Fix:** pass `HB_OT_TAG_GPOS` in the second `hb_ot_layout_table_get_script_tags` call.
- **TeXyoN:** takes the longer list (`get_larger_script_list_table_ot` in `xetex_layout`), so
  fontspec sees the scripts. Listed in `tests/upstream/known.tsv`. Not reported yet.

## 19. Interword space shaping reads the width of a null `font_glue`

- **Where:** the `\XeTeXinterwordspaceshaping` block, lines 24588-24589, `width(font_glue[main_f])`.
- **Bug:** `font_glue[f]` is only built when `app_space` uses the font's own interword glue. While
  `\spaceskip` is nonzero it can stay null, and the block reads `width(null)` without a check.
  The C build crashes.
- **Repro:** `\font\lm="[lmroman12-regular]" \lm \spaceskip=4pt \XeTeXinterwordspaceshaping=1 To
  We \bye` segfaults in TeX Live 2025 xetex. Without `\spaceskip` it runs.
- **Fix:** compare against `param(space_code)(main_f)`, which is the width `font_glue` gets when it
  is built.
- **TeXyoN:** uses the font's space parameter (`shape_interword_space`), so the kern is what
  upstream gives whenever `font_glue` exists. Test `mainloop_spaceskip_shaping` in
  `tests/tex-outputs.rs`. Not reported yet.

## 20. `\font` compares a native font's type flag as an area string

- **Where:** `new_font`, line 28327, `str_eq_str(font_area[f],cur_area)`.
- **Bug:** a native font's `font_area` is `aat_font_flag` or `otgr_font_flag`, not a string. When
  the requested name has an area and matches a native font's name, `str_eq_str` reads the flag as
  a string: its length is that of the printed character (8 for `"FFFE`), and with an area of that
  length it reads `str_start` below the array.
- **Repro:** after a native font is loaded, `\font\b="[assets/lmroman12-regular]"` (an area of 8
  characters). TeX Live 2025 xetex reads out of bounds without a visible effect here.
- **Fix:** for a native font, the area matches only when `cur_area` is empty.
- **TeXyoN:** compares that way (`main_control/assign.rs`), test `font_area_native` in
  `tests/tex-outputs.rs`. It panicked before (TeX Live doc biblatex-gost, through fontspec's `Name/B` requests).
  Not reported yet.

## 22. `\XeTeXpdffile` ignores an inherited `/Rotate`

- **Where:** `pdf_get_rect` in `pdfimage.cpp` (TeX Live 2025), line 128,
  `ppdict_get_int(pdict, "Rotate", &RotAngle)`.
- **Bug:** the boxes come from `ppdict_get_box`, which walks up the page tree, but `/Rotate` is
  read from the page dictionary alone. `/Rotate` is inheritable, and xdvipdfmx rotates the page by
  the inherited value when it draws it, so the TeX box keeps the unrotated size of a page that is
  drawn turned.
- **Repro:** a one-page PDF with `/Rotate 90` on its `/Pages` node, MediaBox 100 by 202.87 bp.
  TeX Live 2025 xetex gives `\XeTeXpdffile` a box 100.375pt wide and 203.63077pt high; with the
  `/Rotate` on the page it gives 203.63077pt by 100.375pt.
- **Fix:** read `/Rotate` through the parents as well, as the boxes are.
- **TeXyoN:** inherits it as the PDF spec says (`pdf_doc_get_page_rect` in `pdf_io`), test `pdf_box` in
  `tests/tex-outputs.rs`. Not reported yet.
