#!/usr/bin/env bash
# Profile-guided optimized build of the texyon CLI. See tools/pgo/README.md.
#
#   tools/pgo/pgo.sh [all|gen|train|use] [extra cargo build args]
#
# Env: PGO_TARGET_DIR (default target/pgo), PGO_PROFILE (use a given .profdata
# instead of training), PGO_WARN=1 (report functions without profile data).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STAGE="${1:-all}"; shift || true
OUT="${PGO_TARGET_DIR:-$ROOT/target/pgo}"
RAW="$OUT/raw"
PROFILE="${PGO_PROFILE:-$OUT/merged.profdata}"
HOST="$(rustc -vV | sed -n 's/^host: //p')"
GEN_BIN="$OUT/gen/$HOST/release/texyon"
USE_BIN="$OUT/use/$HOST/release/texyon"
CORPUS="${TEXYON_CORPUS_DIR:-$HOME/.cache/tectonic-corpus}/raw"

die() { echo "pgo: $*" >&2; exit 1; }

profdata() {
  local bin
  bin="$(rustc --print sysroot)/lib/rustlib/$HOST/bin/llvm-profdata"
  [ -x "$bin" ] || die "llvm-profdata missing, run: rustup component add llvm-tools"
  local want have
  want="$(rustc -vV | sed -n 's/^LLVM version: \([0-9]*\).*/\1/p')"
  have="$("$bin" --version | sed -n 's/.*LLVM version \([0-9]*\).*/\1/p')"
  [ "$want" = "$have" ] || die "llvm-profdata is LLVM $have but rustc uses LLVM $want"
  echo "$bin"
}

# --target keeps RUSTFLAGS off build scripts and proc macros.
build() { # build <target-subdir> <rustflags> [cargo args]
  local dir="$1" flags="$2"; shift 2
  (cd "$ROOT" && RUSTFLAGS="${RUSTFLAGS:-} $flags" cargo build --release \
    --bin texyon --target "$HOST" --target-dir "$OUT/$dir" "$@")
}

gen() {
  build gen "-Cprofile-generate=$RAW" "$@"
}

train() {
  [ -x "$GEN_BIN" ] || die "no instrumented binary, run the gen stage first"
  rm -rf "$RAW"; mkdir -p "$RAW"
  local work line doc
  work="$(mktemp -d)"
  while read -r line; do
    case "$line" in ''|'#'*) continue;; corpus:*) doc="$CORPUS/${line#corpus:}";; *) doc="$ROOT/$line";; esac
    if [ ! -f "$doc" ]; then echo "pgo: skip missing $line"; continue; fi
    echo "pgo: train $line"
    rm -rf "$work/out"; mkdir -p "$work/out"
    (cd "$(dirname "$doc")" && LLVM_PROFILE_FILE="$RAW/%m_%p.profraw" \
      "$GEN_BIN" compile --untrusted -o "$work/out" "$doc" > "$work/log" 2>&1) \
      || { tail -20 "$work/log"; die "training run failed: $line"; }
  done < "$ROOT/tools/pgo/train.txt"
  rm -rf "$work"
  "$(profdata)" merge -o "$OUT/merged.profdata" "$RAW"
  echo "pgo: profile $OUT/merged.profdata"
}

use() {
  [ -f "$PROFILE" ] || die "no profile at $PROFILE"
  local flags="-Cprofile-use=$PROFILE"
  [ "${PGO_WARN:-0}" = 1 ] && flags="$flags -Cllvm-args=-pgo-warn-missing-function"
  build use "$flags" "$@"
  echo "pgo: optimized binary $USE_BIN"
}

case "$STAGE" in
  all) gen "$@"; train; use "$@";;
  gen) gen "$@";;
  train) train;;
  use) use "$@";;
  *) die "unknown stage $STAGE (all|gen|train|use)";;
esac
