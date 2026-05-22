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

## D1b — positional divergence in native text — OPEN

After D1a, 21/38 documents still diverge, now in glyph *position* values
(not missing chars): e.g. `1302.4270v3` has 0 warnings but its `.xdv`
still differs from the oracle in native-glyph x-positions (~45k bytes).
Affects long native-text documents (lshort, most arXiv); classic-TFM and
short native docs pass. Next target. Likely a metric/measurement
difference in native glyph positioning rather than a missing-char issue.

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
