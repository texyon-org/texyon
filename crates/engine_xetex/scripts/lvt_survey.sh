#!/usr/bin/env bash
# lvt_survey.sh: find the LaTeX .lvt tests that run to END-TEST-LOG, to
# refresh tests/corpus/lvt-viable.txt. Results land in $RES.
ROOT="$(git rev-parse --show-toplevel)"
BIN="${BIN:-$ROOT/target/release/tectonic}"
CACHE="${TECTONIC_CORPUS_DIR:-$HOME/.cache/tectonic-corpus}/raw/latex-tests"
SUP="$CACHE/support"
OUT="${OUT:-/tmp/lvt_survey}"; rm -rf "$OUT"; mkdir -p "$OUT"
RES="${RES:-/tmp/lvt_results.tsv}"; > "$RES"
for dir in testfiles testfiles-TU; do
  for f in "$CACHE/base/$dir"/*.lvt; do
    [ -e "$f" ] || continue
    stem=$(basename "$f" .lvt); wd="$OUT/$dir/$stem"; mkdir -p "$wd"; cp "$f" "$wd/$stem.tex"
    SOURCE_DATE_EPOCH=0 LC_ALL=C TZ=UTC timeout 40 "$BIN" -X compile --keep-logs \
      -Z deterministic-mode -Z continue-on-errors -Z search-path="$SUP" \
      -o "$wd" "$wd/$stem.tex" >"$wd/run.txt" 2>&1
    if grep -q "END-TEST-LOG" "$wd/$stem.log" 2>/dev/null; then
      echo -e "$dir/$stem\tVIABLE" >> "$RES"
    else
      echo -e "$dir/$stem\tFAIL" >> "$RES"
    fi
    rm -f "$wd"/*.xdv 2>/dev/null
  done
done
echo "DONE" >> "$RES"
