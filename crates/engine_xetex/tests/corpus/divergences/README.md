# Known divergences caught by the parity harness

Reproducers where the **current Rust engine** diverges from the
**pre-Rust C oracle** (`oracle-pre-rust`, commit 1a99fcf5). These are
the engine's to-fix list — the in-tree `tex-outputs` goldens miss them
because those goldens were blessed *from* the Rust engine.

## D1a — spurious U+0000 in native-font line breaking — FIXED (90acc757)

**Root cause:** `new_native_character`'s font-mapping branch passed
`mem_as_u16_ptr_mut(start)` (a pointer into `mem[]`) to `apply_mapping`
instead of a pointer into `str_pool`, so every char built via that path
mapped to 0. Surfaced through hyphenation (the discretionary hyphen) and
math native characters. Fix: map from `str_pool_ptr(start)`.

**Result:** `native_linebreak_nul.tex` 56 warnings → 0, xdv now identical
to oracle; document parity 12/38 → 16/38; TRIP 2/2, tex-outputs 29/29.

**Repro (now passes):** `native_linebreak_nul.tex` (a wrapping justified
paragraph in `lmroman10-regular.otf` with `Ligatures=TeX`).

## D1b — divergent line breaking at explicit hyphens in native words — FIXED

**Symptom (as originally characterized):** "positional divergence" — long
native-text documents (lshort, most arXiv) and a handful of `.lvt` tests
diverged from the oracle. The real cause was not glyph *metrics* but
*line-breaking*: words containing an explicit hyphen/dash (`lig-kern`,
`reconstitute/lig-kern`) broke at different points, and the candidate
emitted a **doubled hyphen** glyph at the break (the explicit hyphen plus
an extra one from the line-breaker auto-hyphenating the un-split word).

**Root cause:** in `main_control_main_loop` (`src/scan.rs`), the native
(AAT/OTGR) path that splits a native word at explicit hyphens into
segments separated by *empty* discretionaries was gated on
`cur_list_mode() == 1` — but `1` is VMODE; HMODE is `104`. The branch
therefore never ran in horizontal mode, and its body was an incomplete
stub (`break` placeholder + TODO). All native HMODE words fell through to
the MMODE single-node path, so a hyphenated word became one un-split
native node; the line-breaker's native-hyphenation path (`hyphenate`)
then auto-hyphenated it, doubling the hyphen at explicit-hyphen breaks.
Space-separated words were unaffected (each is already its own run), which
is why the bug only surfaced on words with embedded hyphens/dashes.

**Fix:** corrected the mode test to `== 104` (HMODE) and faithfully
translated the C `do { ... } while (main_k != 0)` hyphen-splitting loop
(oracle `xetex-xetex0.c:17671–17778`), inserting an empty `new_disc()`
after each hyphen segment.

**Result:** full corpus `corpus.sh check all` → **638 passed, 0 diverged**
(was: docs 16/38, lvt 596/601). The 5 residual `.lvt` divergences below
(tlxs01l/p, tu-textcomp-003, vtl2e06, properties-006-warn) all resolved.
TRIP 2/2, tex-outputs 29/29, crate unit tests 26/26.

**Repro (now passes):** `aaaaaaaa ... long-compound-word` forced to break
at the second hyphen; `tier1/ligatures_kerning.tex` (the
`reconstitute/lig-kern` paragraph).

## D2 — `bidi_arabic` segfault — FIXED

`tier1/bidi_arabic.tex` (and any RTL paragraph — a single Arabic word in
a `\begin{Arabic}` block sufficed) segfaulted in the candidate where the
oracle compiled cleanly.

**Root cause:** `reverse` (`src/shipout.rs`) read and wrote a glue node's
glue-spec pointer via `mem[p+1].b32.s1`, but `GLUE_NODE_glue_ptr(p)` is
`b32.s0` (`b32.s1` is `leader_ptr`, which is `TEX_NULL` for normal glue).
So `g` came back null and `BOX_width(g)` → `mem[g+1]` dereferenced an
invalid address. The working `hlist_out`/`vlist_out` glue handlers
already used `b32.s0`; only the session-114 `reverse` translation had the
field swapped (read site + the leaders-glue write site).

**Fix:** use `mem_b32s0(p + 1)` for the glue_ptr read and
`set_mem_b32s0(p + 1, g)` for the leaders write, matching
`xetex-shipout.c:1555,1603`.

**Result:** `bidi_arabic.tex` compiles; its `.xdv` is byte-identical to
the oracle. Full corpus `corpus.sh check all` → **639 passed, 0 diverged**
(bidi_arabic now counted, was NOXDV). TRIP 2/2, tex-outputs 29/29.

---
### (historical) D1 — original combined symptom

**Symptom:** the candidate emits many
`Missing character: There is no ^^@ (U+0000) in font [...]/mapping=tex-text;`
warnings (oracle: 0) and the `.xdv` differs (~29% of bytes on real
documents). One null is injected per native word that is split at a line
break.

**Trigger:** native (OpenType) font + a paragraph long enough to break
across lines. Short lines, math, classic TFM fonts, and HarfBuzz
`Script=` runs (CJK/Devanagari) are unaffected — only the
`mapping=tex-text` native-word-node line-break path.

**Likely location:** the native-word-node split/rebuild in line breaking
— `post_line_break` / `reconstitute` / native-word-node copy
(translated sessions 76, 79). An off-by-one in the word-node character
buffer length when splitting at a break appends a trailing NUL.

**Scope at discovery:** 25 of 38 corpus documents diverged on this
(every doc with wrapping native-font prose: unicode_text,
ligatures_kerning, lshort, and 21 arXiv papers).

**Verify:**
```sh
ORACLE=../../../../tectonic-oracle/target/release/tectonic
for b in "$ORACLE" target/release/tectonic; do
  SOURCE_DATE_EPOCH=0 "$b" -X compile --outfmt xdv -Z deterministic-mode \
    -o /tmp/$$ divergences/native_linebreak_nul.tex 2>&1 | grep -c 'Missing character'
done   # oracle prints 0, candidate prints >0
