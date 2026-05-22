# Known divergences caught by the parity harness

Reproducers where the **current Rust engine** diverges from the
**pre-Rust C oracle** (`oracle-pre-rust`, commit 1a99fcf5). These are
the engine's to-fix list — the in-tree `tex-outputs` goldens miss them
because those goldens were blessed *from* the Rust engine.

## D1 — spurious U+0000 in native-font line breaking

**Repro:** `native_linebreak_nul.tex` (a wrapping justified paragraph in
`lmroman10-regular.otf` with `Ligatures=TeX`).

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
