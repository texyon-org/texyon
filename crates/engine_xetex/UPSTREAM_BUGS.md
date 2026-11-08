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
