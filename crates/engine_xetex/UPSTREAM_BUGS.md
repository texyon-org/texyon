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
- **TeXyoN:** keeps the upstream behaviour, test `mark_classes_carry_over_like_xetex`. Flip it
  together with upstream.

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
