#!/usr/bin/env bash
# Usage: plan/scripts/parity.sh [label]
#
# Runs the tectonic tex-outputs parity tests and appends a line to
# crates/engine_xetex/plan/parity-history.log.  The `label` is the
# free-text tag for the entry ("before" / "after" / "session-5-start"
# / …). Defaults to "run".
#
# See plan/05-testing.md §"The parity harness script" for context.

set -euo pipefail

label="${1:-run}"
ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
repo_root="$(git rev-parse --show-toplevel)"
out="$(mktemp -t tectonic-parity.XXXXXX)"
trap 'rm -f "$out"' EXIT

cd "$repo_root"

# --no-fail-fast so one bad case doesn't hide the rest of the count.
cargo test -p tectonic --test tex-outputs --no-fail-fast 2>&1 | tee "$out"

# The final summary line looks like:
#   test result: ok. N passed; M failed; ...
# or   test result: FAILED. N passed; M failed; ...
result="$(grep -E '^test result:' "$out" | tail -1 || true)"
if [[ -z "$result" ]]; then
    result="(no summary line produced — build/link failure?)"
fi

log="$repo_root/crates/engine_xetex/plan/parity-history.log"
printf '%s  %-20s  %s\n' "$ts" "$label" "$result" >> "$log"
echo
echo "Recorded to $log:"
tail -1 "$log"
