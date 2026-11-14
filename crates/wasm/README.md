# texyon_wasm

The engine and the live preview as a `wasm32-wasip1` module, for the browser
through [`packages/wasm`](../../packages/wasm) and for VS Code where no native
binary exists.

## Building

```sh
rustup target add wasm32-wasip1
cargo xtask deps --mupdf      # wasi-sdk and the C libraries, into deps/
cargo xtask pack              # the support pack, into target/wasm-assets
cargo xtask wasm --pgo        # the module, beside it
cargo xtask test wasm         # a live-preview session against it under Node
```

`--pgo` builds a module that counts, replays editing sessions on the training
documents in `crates/xtask/corpus` and the engine's tier1 corpus, and builds the
release module from that profile. Typesetting gets about 5-8% faster. It needs
`rustup component add llvm-tools`. Leave it out for a quicker build.

rustup ships no profiler runtime for wasm, so the counting module links
[minicov](https://crates.io/crates/minicov) through the `pgo` feature and hands
its counters to the host through `texyon_pgo_profile`. minicov's raw format
follows one LLVM version. When stable rustc moves to a new LLVM, `cargo xtask
wasm --pgo` says so, and minicov needs the matching release. A profile of the
native binary does not transfer: the hot engine functions compile to a different
control flow on 32-bit wasm, and LLVM discards their counts.

## Why `wasm32-wasip1`

The engine still has C and C++ (HarfBuzz, pdf_io, TECkit and the XeTeX shims,
plus MuPDF for rendering), which needs a libc and rules out
`wasm32-unknown-unknown`. Emscripten brings its own JavaScript glue, and its
answer to synchronous I/O is Asyncify, which instruments every function on the
stack. wasi-sdk gives a libc and a libc++ and nothing else: real I/O is answered
by the host through two imports, and the WASI calls the module still makes are
shimmed in [`packages/wasm/src/wasi.ts`](../../packages/wasm/src/wasi.ts).

## Things that fail late and quietly

**setjmp/longjmp needs `-lsetjmp`.** TeX reports fatal errors by longjmp'ing
out of the engine, which on wasm is the exception-handling proposal: compiled
with `-mllvm -wasm-enable-sjlj`, linked with wasi-sdk's `libsetjmp.a`. Without
the library `rust-lld` does not fail, it turns `__wasm_setjmp` and
`__c_longjmp` into imports no host can supply, and the module fails at
`WebAssembly.instantiate`. `cargo xtask wasm` refuses such a module.

**`malloc` is ours.** XeTeX `calloc`s about 275 MB of arenas before it
typesets anything, and wasi-libc memsets every byte.
[`src/alloc_shim.rs`](src/alloc_shim.rs) defines the whole family on
`dlmalloc` and serves large zeroed blocks from fresh pages, about 30% off a cold
compile. It must define every entry point: one left to wasi-libc hands out
blocks our `free` corrupts the heap releasing.

**wasm checks indirect call signatures.** TECkit's status type is `long`,
64-bit on Linux and 32-bit here. Declared as `i64` it trapped, `libc::c_long`
is right everywhere.

**Two copies of a library link silently.** `--allow-multiple-definition` lets
the allocator shim win, so a duplicate symbol is not an error but a pick of
whichever came first. MuPDF bundles FreeType, HarfBuzz and zlib, and TeXyoN's
`hb_font_create` once resolved to MuPDF's `hb_face_set_user_data` and trapped in
font setup. MuPDF is therefore built against TeXyoN's copies, and the `pdf_*`
names both define are renamed in the preprocessor, since `llvm-objcopy` cannot
rename wasm symbols.

**A build script must declare its inputs.** `preview_render` lists the MuPDF
archives as `rerun-if-changed`. Before it did, a rebuilt MuPDF kept the old link.

## C libraries

A browser has no system libraries, so **libpng**, **FreeType** and
**Graphite2** are compiled from source in `deps/wasm`, which the bridge crates'
build scripts use whenever the target family is wasm. **MuPDF** is built by
`cargo xtask deps --mupdf`, separately because it is 145 MB of source under the
AGPL. WASI has no `mkstemp`, so its temp-file helper throws instead.

Two are not vendored:

- **fontconfig** has nothing to find in a browser. The crate supplies the API
  and reports no match. Fonts named by file, such as fontspec's `Path=`, work.
- **ICU** would mean about 30 MB of locale data. XeTeX uses thirteen of its
  functions, reimplemented on `unicode-bidi`, `icu_segmenter` and `encoding_rs`
  in [`crates/bridge_icu/src/wasm_shim.rs`](../bridge_icu/src/wasm_shim.rs).
