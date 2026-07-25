#!/usr/bin/env bash
# Build the Tectonic WebAssembly module and drop it where the client expects it.
#
# Usage: tools/wasm/build.sh [--debug] [output-dir]
#        output-dir defaults to client/public/tectonic
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"

PROFILE=release
PROFILE_DIR=release
if [ "${1:-}" = "--debug" ]; then
    PROFILE=dev
    PROFILE_DIR=debug
    shift
fi

OUT="${1:-$REPO/client/public/tectonic}"

# shellcheck source=/dev/null
source "$HERE/env.sh"

if [ ! -d "$HERE/vendor/freetype" ]; then
    echo "vendored C sources are missing; running fetch-deps.sh" >&2
    "$HERE/fetch-deps.sh"
fi

echo "== building tectonic_wasm ($PROFILE)"
cd "$REPO"
cargo build --profile "$PROFILE" --target wasm32-wasip1 -p tectonic_wasm

MODULE="$REPO/target/wasm32-wasip1/$PROFILE_DIR/tectonic_wasm.wasm"

# The linker turns an unresolved __wasm_setjmp / __c_longjmp into an *import*
# rather than an error, and an imported tag is something no host can supply — so
# a module missing -lsetjmp links cleanly and then fails to instantiate in the
# browser. Catch it here instead of in a devtools console.
echo "== checking imports"
python3 - "$MODULE" <<'PY'
import sys

data = open(sys.argv[1], "rb").read()
assert data[:4] == b"\0asm", "not a wasm module"

pos = 8


def uleb(i):
    result = shift = 0
    while True:
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return result, i
        shift += 7


bad = []
while pos < len(data):
    section = data[pos]
    pos += 1
    size, pos = uleb(pos)
    end = pos + size

    if section == 2:  # imports
        count, j = uleb(pos)
        for _ in range(count):
            length, j = uleb(j)
            module = data[j : j + length].decode()
            j += length
            length, j = uleb(j)
            name = data[j : j + length].decode()
            j += length
            kind = data[j]
            j += 1
            if kind == 0:
                _, j = uleb(j)
            elif kind == 1:
                j += 1
                j += 1
                _, j = uleb(j)
            elif kind == 2:
                j += 1
                _, j = uleb(j)
            elif kind == 3:
                j += 2
            elif kind == 4:
                j += 1
                _, j = uleb(j)

            if module not in ("wasi_snapshot_preview1", "env"):
                bad.append(f"{module}::{name} (unexpected module)")
            elif module == "env" and not name.startswith("tectonic_host_"):
                bad.append(f"{module}::{name} (not a Tectonic host import)")

    pos = end

if bad:
    print("error: the module imports things no host can provide:", file=sys.stderr)
    for item in bad:
        print(f"  {item}", file=sys.stderr)
    print(
        "\nAn imported __c_longjmp tag means -lsetjmp was missing from RUSTFLAGS;\n"
        "see tools/wasm/env.sh.",
        file=sys.stderr,
    )
    sys.exit(1)

print("   imports are clean")
PY

mkdir -p "$OUT"

if command -v wasm-opt >/dev/null 2>&1 && [ "$PROFILE" = "release" ]; then
    echo "== wasm-opt"
    # The engine relies on the exception-handling proposal for setjmp/longjmp,
    # and bulk memory for its arena clears, so both have to stay enabled.
    wasm-opt -O2 --enable-exception-handling --enable-bulk-memory \
        "$MODULE" -o "$OUT/tectonic_wasm.wasm"
else
    cp "$MODULE" "$OUT/tectonic_wasm.wasm"
fi

echo "== done: $OUT/tectonic_wasm.wasm ($(du -h "$OUT/tectonic_wasm.wasm" | cut -f1))"

if [ ! -f "$OUT/manifest.json" ]; then
    echo
    echo "No support pack yet. Run: tools/wasm/build-assets.sh"
fi
