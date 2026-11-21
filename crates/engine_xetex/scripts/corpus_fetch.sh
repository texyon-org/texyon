#!/usr/bin/env bash
# corpus_fetch.sh: (re)populate the out-of-repo parity corpus cache.
#
# The committed corpus consists of:
#   * Authored Tier 1 docs in crates/engine_xetex/tests/corpus/tier1/  (in repo)
#   * This script + manifest.tsv, which reproduce everything else into
#     $TEXYON_CORPUS_DIR (NOT in repo; large + third-party licensing).
#
# Sources:
#   * Local TeX Live doc tree (redistributable real-world LaTeX sources).
#   * arXiv e-print tarballs (per-ID, via the public e-print endpoint).
#
# Usage:
#   TEXYON_CORPUS_DIR=~/.cache/texyon-corpus ./corpus_fetch.sh [tl|arxiv|all]
#
# Bulk arXiv (thousands of papers) is NOT done here.
set -euo pipefail

CORPUS_DIR="${TEXYON_CORPUS_DIR:-$HOME/.cache/texyon-corpus}"
RAW="$CORPUS_DIR/raw"
TL="${TEXLIVE_DOC:-/usr/local/texlive/2026/texmf-dist/doc}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$HERE/../tests/corpus/manifest.tsv"
mode="${1:-all}"

mkdir -p "$RAW"

fetch_tl() {
  echo ">> harvesting redistributable sources from TeX Live ($TL)"
  # lshort: book-length real-world LaTeX
  if [ -f "$TL/latex/lshort-english"/lshort-*.src.tar.gz ]; then
    mkdir -p "$RAW/lshort"
    tar xzf "$TL"/latex/lshort-english/lshort-*.src.tar.gz \
        -C "$RAW/lshort" --strip-components=1
  fi
  # unicode-math example documents (XeLaTeX, OpenType math)
  mkdir -p "$RAW/unicode-math"
  cp "$TL"/latex/unicode-math/unimath-example.ltx \
     "$TL"/latex/unicode-math/unimath-symbols.ltx "$RAW/unicode-math/" 2>/dev/null || true
  # polyglossia manual (multilingual / bidi)
  mkdir -p "$RAW/polyglossia"
  cp "$TL"/latex/polyglossia/polyglossia.tex "$RAW/polyglossia/" 2>/dev/null || true
}

# TeX Live package/engine manuals. PENDING: 0/7 compiled under the pinned
# 2022 bundle (manual-only assets, fonts not in bundle, dvipdfm driver,
# newer-kernel commands). Kept staged so they can be re-surveyed when the
# bundle is upgraded, several should start compiling. They are in the
# manifest as `tier2-pending` and are NOT part of the active parity set
# until a survey under the new bundle promotes them to `tier2`.
fetch_manuals() {
  echo ">> harvesting package/engine manuals from TeX Live ($TL)"
  while IFS='|' read -r id sub main; do
    [ -z "$id" ] && continue
    dst="$RAW/manuals/$id"
    if [ -d "$dst" ]; then echo "   have $id"; continue; fi
    mkdir -p "$dst"
    cp -r "$TL/$sub/." "$dst/" 2>/dev/null || true
    [ -f "$dst/$main" ] && echo "   harvested $id" || echo "   MISS $id ($main)"
  done <<'LIST'
pgfmanual|generic/pgf|pgfmanual.tex
memoir|latex/memoir|memman.tex
beamer-guide|latex/beamer|beameruserguide.tex
latex-usrguide|latex/base|usrguide.tex
xetex-reference|xetex/xetexref|xetex-reference.tex
font-change-xetex|xetex/font-change-xetex|font-change-xetex.tex
xetex-notes|xetex/base|XeTeX-notes.tex
LIST
}

fetch_lvt() {
  # LaTeX kernel regression test suite (.lvt): engine edge-case coverage.
  # These \input{regression-test} and \showbox/\showlists engine internals
  # into the .log; the PARITY SIGNAL IS THE .log, not the xdv. Run them with
  # -Z continue-on-errors -Z search-path="$RAW/latex-tests/support" (NOT
  # --untrusted, which disables search-path). A test "ran" iff its log
  # contains END-TEST-LOG.
  echo ">> cloning LaTeX2e test suite into $RAW/latex-tests"
  local dst="$RAW/latex-tests"
  if [ -d "$dst/base" ]; then echo "   have latex-tests"; return; fi
  local tmp; tmp="$(mktemp -d)"
  git clone --depth 1 https://github.com/latex3/latex2e "$tmp/latex2e" 2>&1 | tail -2
  mkdir -p "$dst"
  cp -r "$tmp/latex2e/base" "$dst/base"
  cp -r "$tmp/latex2e/support" "$dst/support"
  rm -rf "$tmp"
  echo "   staged $(find "$dst/base" -name '*.lvt' | wc -l) .lvt + support files"
  # NOTE: latex3/l3kernel testfiles can be added the same way once the
  # base set is wired through the harness.
}

fetch_arxiv() {
  echo ">> fetching arXiv e-print sources listed in $MANIFEST"
  # manifest columns: tier  id  main_relpath  source_url
  awk -F'\t' '$1=="tier2" && $4 ~ /arxiv/ {print $2"\t"$4}' "$MANIFEST" 2>/dev/null \
  | while IFS=$'\t' read -r id url; do
      dest="$RAW/arxiv/$id"
      if [ -d "$dest" ]; then echo "   have $id"; continue; fi
      mkdir -p "$dest"
      echo "   fetching $id"
      if curl -sSL -m 60 "$url" -o "$dest/src.tar.gz"; then
        # e-print payloads are gzip; usually a tar, sometimes a single .tex.gz
        if tar tzf "$dest/src.tar.gz" >/dev/null 2>&1; then
          tar xzf "$dest/src.tar.gz" -C "$dest"
        else
          gunzip -c "$dest/src.tar.gz" > "$dest/main.tex" 2>/dev/null || true
        fi
      fi
      sleep 3   # be polite to arXiv
    done
}

case "$mode" in
  tl)      fetch_tl ;;
  manuals) fetch_manuals ;;
  lvt)     fetch_lvt ;;
  arxiv)   fetch_arxiv ;;
  all)     fetch_tl; fetch_manuals; fetch_lvt; fetch_arxiv ;;
  *) echo "usage: $0 [tl|manuals|lvt|arxiv|all]"; exit 2 ;;
esac
echo ">> done. corpus cache at $CORPUS_DIR"
