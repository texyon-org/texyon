# Source this to build Tectonic for wasm32-wasip1.
#
#     source tools/wasm/env.sh
#     cargo build --release --target wasm32-wasip1 -p tectonic_wasm
#
# Prerequisites: tools/wasm/fetch-deps.sh has been run, and wasi-sdk is unpacked
# next to this file (see tools/wasm/README.md).

_tectonic_wasm_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"

export WASI_SDK="${WASI_SDK:-$_tectonic_wasm_dir/wasi-sdk-33.0-x86_64-linux}"
export WASI_SYSROOT="$WASI_SDK/share/wasi-sysroot"

if [ ! -d "$WASI_SYSROOT" ]; then
    echo "tools/wasm/env.sh: no wasi-sdk at $WASI_SDK" >&2
    echo "  see tools/wasm/README.md" >&2
fi

export CC_wasm32_wasip1="$WASI_SDK/bin/clang"
export CXX_wasm32_wasip1="$WASI_SDK/bin/clang++"
export AR_wasm32_wasip1="$WASI_SDK/bin/llvm-ar"

# TeX reports fatal errors by longjmp'ing out of the engine. wasm can only do
# that through the exception-handling proposal, which every current browser
# implements but which clang still needs to be told to use.
_tectonic_wasm_cflags="--sysroot=$WASI_SYSROOT --target=wasm32-wasip1 -mllvm -wasm-enable-sjlj"

export CFLAGS_wasm32_wasip1="$_tectonic_wasm_cflags"
export CXXFLAGS_wasm32_wasip1="$_tectonic_wasm_cflags"

# Harfbuzz and Graphite2 are C++, so the link needs wasi-sdk's libc++ — the
# no-exceptions build, to match the -fno-exceptions that cc-rs compiles with.
# Rust supplies libc itself and knows nothing about the rest of the sysroot.
#
# -lsetjmp is not optional, and its absence does not look like an error: Rust's
# own wasi sysroot has no libsetjmp.a, and rust-lld responds to the resulting
# undefined __wasm_setjmp / __c_longjmp by turning them into imports — including
# an imported *tag*, which no host can supply. The module then links cleanly and
# fails at WebAssembly.instantiate. tools/wasm/build.sh checks for this.
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }\
-L native=$WASI_SYSROOT/lib/wasm32-wasip1 \
-L native=$WASI_SYSROOT/lib/wasm32-wasip1/noeh \
-C link-arg=-lsetjmp \
-C link-arg=--allow-multiple-definition \
-C link-arg=--initial-memory=67108864 \
-C link-arg=--max-memory=2147483648"

# --allow-multiple-definition lets crates/wasm/src/alloc_shim.rs take over
# malloc/calloc/realloc/free from wasi-libc, whose allocator never grows the
# WebAssembly memory and so caps the engine at whatever --initial-memory gives
# it. Rust objects are linked ahead of the sysroot, so ours is the definition
# that wins; tools/wasm/smoke-test.mjs would fail on a 76 MB arena if it did not.

unset _tectonic_wasm_dir _tectonic_wasm_cflags
