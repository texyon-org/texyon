#!/usr/bin/env bash
# corpus.sh: byte-parity gate for engine_xetex.
#
#   bless : run ORACLE_BIN over the corpus and store output hashes as goldens.
#   check : run the CANDIDATE (current engine) and compare to the goldens.
#
# For an A/B against a baseline build, bless the baseline into a scratch
# CORPUS_GOLDENS dir, then check the candidate against it.
#
# Two artifact kinds:
#   * documents (manifest tier1/tier1-real/tier2)  -> parity on the .xdv bytes
#   * regression tests (lvt-viable.txt)            -> parity on the .log
#     (normalized: banner + abs paths stripped; these rarely ship a page)
#
# Usage:
#   ORACLE_BIN=<baseline>/tectonic [CORPUS_GOLDENS=dir] \
#     crates/engine_xetex/scripts/corpus.sh bless [scope]
#   [CANDIDATE_BIN=...] [CORPUS_GOLDENS=dir] crates/engine_xetex/scripts/corpus.sh check [scope]
#   scope = all (default) | docs | lvt | tier1 | tier1-real | tier2
#
# Determinism (both engines): SOURCE_DATE_EPOCH=0, LC_ALL=C, TZ=UTC,
# -Z deterministic-mode, one process per input, and the TL2022 bundle the
# goldens were blessed against (CORPUS_BUNDLE).
set -uo pipefail

MODE="${1:-check}"
SCOPE="${2:-all}"
ROOT="$(git rev-parse --show-toplevel)"
CORPUS="$ROOT/crates/engine_xetex/tests/corpus"
CACHE="${TEXYON_CORPUS_DIR:-$HOME/.cache/tectonic-corpus}/raw"
SUP="$CACHE/latex-tests/support"
GOLD="${CORPUS_GOLDENS:-$CORPUS/goldens}"; mkdir -p "$GOLD"
MANIFEST="$CORPUS/manifest.tsv"
LVTLIST="$CORPUS/lvt-viable.txt"
DOCS_SHA="$GOLD/docs.sha"
LVT_SHA="$GOLD/lvt.sha"
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT
FAILDIR="/tmp/corpus_fail"; rm -rf "$FAILDIR"; mkdir -p "$FAILDIR"

if [ "$MODE" = bless ]; then
  BIN="${ORACLE_BIN:?bless requires ORACLE_BIN (the reference binary)}"
  echo "BLESS using oracle: $BIN"
else
  BIN="${CANDIDATE_BIN:-$ROOT/target/release/texyon}"
  echo "CHECK using candidate: $BIN"
fi
[ -x "$BIN" ] || { echo "engine binary not executable: $BIN"; exit 2; }

run_env() { SOURCE_DATE_EPOCH=0 LC_ALL=C TZ=UTC "$@"; }
BUNDLE="${CORPUS_BUNDLE:-https://relay.fullyjustified.net/default_bundle_v33.tar}"

# normalize a .log: drop the engine banner, any Tectonic notes, and the
# (run-specific) absolute work path. Everything else must match.
norm_log() { sed -e '/^This is /d' -e '/^entering extended mode/d' \
                 -e "s#${WORK//#/\\#}[^ ]*##g" "$1"; }

want() { # want <tier>  -> 0 if this tier is in scope
  case "$SCOPE" in
    all) return 0;;
    docs) [ "$1" != lvt ];;
    lvt) [ "$1" = lvt ];;
    *) [ "$1" = "$SCOPE" ];;
  esac
}

pass=0; fail=0; err=0; faillist=""

emit() { # emit <key> <hash> <store>
  if [ "$MODE" = bless ]; then
    printf '%s\t%s\n' "$1" "$2" >> "$3.tmp"
  else
    local want; want="$(grep -m1 -P "^\Q$1\E\t" "$3" 2>/dev/null | cut -f2)"
    if [ -z "$want" ]; then echo "  NEW  $1 (no golden)"; err=$((err+1));
    elif [ "$want" = "$2" ]; then pass=$((pass+1));
    else echo "  DIFF $1"; fail=$((fail+1)); faillist="$faillist $1"; fi
  fi
}

# ---- documents -> xdv parity ----
do_docs() {
  [ "$MODE" = bless ] && : > "$DOCS_SHA.tmp"
  while IFS=$'\t' read -r tier id rel src; do
    want "$tier" || continue
    if [ "$tier" = tier1 ]; then main="$CORPUS/$rel"; else main="$CACHE/$rel"; fi
    [ -f "$main" ] || { echo "  MISS $id ($main)"; continue; }
    wd="$WORK/doc/$id"; mkdir -p "$wd"
    run_env "$BIN" -X compile --bundle "$BUNDLE" --outfmt xdv --keep-logs --untrusted \
      -Z deterministic-mode -o "$wd" "$main" >"$wd/run.txt" 2>&1
    xdv="$(ls "$wd"/*.xdv 2>/dev/null | head -1)"
    if [ -z "$xdv" ]; then echo "  NOXDV $id"; continue; fi
    h="$(sha256sum "$xdv" | cut -d' ' -f1)"
    emit "$id" "$h" "$DOCS_SHA"
  done < <(grep -E '^(tier1|tier1-real|tier2)'$'\t' "$MANIFEST")
  [ "$MODE" = bless ] && mv "$DOCS_SHA.tmp" "$DOCS_SHA"
}

# ---- regression tests -> normalized .log parity ----
do_lvt() {
  [ "$MODE" = bless ] && : > "$LVT_SHA.tmp"
  while read -r rel; do
    [ -z "$rel" ] && continue
    f="$CACHE/latex-tests/base/$rel.lvt"
    [ -f "$f" ] || { echo "  MISS $rel"; continue; }
    stem="$(basename "$rel")"; wd="$WORK/lvt/$stem"; mkdir -p "$wd"; cp "$f" "$wd/$stem.tex"
    run_env "$BIN" -X compile --bundle "$BUNDLE" --keep-logs -Z deterministic-mode \
      -Z continue-on-errors -Z search-path="$SUP" -o "$wd" "$wd/$stem.tex" >"$wd/run.txt" 2>&1
    log="$wd/$stem.log"
    [ -f "$log" ] || { echo "  NOLOG $rel"; continue; }
    h="$(norm_log "$log" | sha256sum | cut -d' ' -f1)"
    emit "$rel" "$h" "$LVT_SHA"
  done < <(grep -vE '^#' "$LVTLIST" | grep -E '\S')
  [ "$MODE" = bless ] && mv "$LVT_SHA.tmp" "$LVT_SHA"
}

want docs && do_docs
want lvt  && do_lvt

echo
if [ "$MODE" = bless ]; then
  echo "Blessed goldens:"
  [ -f "$DOCS_SHA" ] && echo "  docs: $(grep -c . "$DOCS_SHA") in $DOCS_SHA"
  [ -f "$LVT_SHA" ]  && echo "  lvt : $(grep -c . "$LVT_SHA") in $LVT_SHA"
else
  echo "PARITY: $pass passed, $fail diverged, $err missing-golden"
  [ -n "$faillist" ] && { echo "Divergences:"; for x in $faillist; do echo "  $x"; done; \
    echo "(re-run that input under ORACLE_BIN and candidate to byte-diff)"; }
  [ "$fail" -gt 0 ] && exit 1 || exit 0
fi
