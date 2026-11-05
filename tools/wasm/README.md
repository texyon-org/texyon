# Building Tectonic for WebAssembly

This directory builds the engine into a `wasm32-wasip1` module that runs LaTeX
in a browser. The TypeScript wrapper is in [`packages/wasm/`](../../packages/wasm), and
the [ZapTeX editor](https://github.com/zaptex-org/editor) is built on it.

## Quick start

```sh
# One-time: the C toolchain and the libraries Tectonic normally takes from the system
tools/wasm/fetch-deps.sh --with-mupdf
curl -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-33/wasi-sdk-33.0-x86_64-linux.tar.gz \
  | tar xz -C tools/wasm
rustup target add wasm32-wasip1

# The module
tools/wasm/build.sh

# The support pack: the format dump and the class, package and font files.
# Needs a native `tectonic` binary, so build one first.
cargo build --release
tools/wasm/build-assets.sh

# Check it actually typesets, without involving a browser
node tools/wasm/smoke-test.mjs
```

## What is in here

| File | Purpose |
| --- | --- |
| `env.sh` | Compiler, linker and flag setup. Source it before `cargo build --target wasm32-wasip1`. |
| `fetch-deps.sh` | Downloads libpng, FreeType and Graphite2 into `vendor/`; `--with-mupdf` also fetches and builds the SVG renderer. |
| `build.sh` | Builds the module, checks its imports, stages it for the demo. |
| `build-assets.sh` | Compiles seed documents natively and packs everything they touched. |
| `smoke-test.mjs` | Runs the module under Node and asserts it draws pages, and redraws none when nothing changed. |

## Why the target is `wasm32-wasip1`

The engine is still around 140k lines of C and C++ — HarfBuzz, pdf_io, TECkit
and the XeTeX shims, plus MuPDF for rendering. That needs a libc, which
rules out `wasm32-unknown-unknown`. Emscripten would work but brings its own JS
glue, which is the opposite of the hand-written TypeScript API here, and its
usual answer for synchronous I/O is Asyncify — instrumenting every function on
the stack, at a cost this project cannot pay.

wasi-sdk gives a libc and a libc++ and nothing else, which is what we want: all
real I/O is answered by the host through two imports, and the WASI syscalls the
module still makes are shimmed in about 200 lines
([`packages/wasm/src/wasi.ts`](../../packages/wasm/src/wasi.ts)).

## Things that were not obvious

Each of these is a trap that fails late and quietly. They are all handled, but
they are worth knowing about before changing the build.

**setjmp/longjmp needs `-lsetjmp`, and its absence does not look like an error.**
TeX reports fatal errors by longjmp'ing out of the engine. On wasm that lowers
to the exception-handling proposal, which needs `-mllvm -wasm-enable-sjlj` when
compiling. It also needs wasi-sdk's `libsetjmp.a` at link time, which Rust's own
wasi sysroot does not ship. Without it `rust-lld` does not fail — it turns
`__wasm_setjmp` and `__c_longjmp` into *imports*, one of them an imported tag,
which no host can supply. The module links cleanly and then fails at
`WebAssembly.instantiate`. `build.sh` checks the import table for exactly this.

**`malloc` is ours, not wasi-libc's.** XeTeX `calloc`s roughly 275 MB of
arenas before it typesets anything, and wasi-libc memsets every byte of them.
[`crates/wasm/src/alloc_shim.rs`](../../crates/wasm/src/alloc_shim.rs) defines
the whole family on `dlmalloc` and serves large zeroed blocks from fresh pages,
which cuts about 30% off a cold compile. Rust's default allocator calls the
same functions, so C and Rust share one heap. The shim must define every
allocation entry point: one left to wasi-libc (as `posix_memalign` once was)
hands out blocks that our `free` would corrupt the heap releasing.

**wasm validates indirect call signatures, and it found a real bug.** TECkit's
status type is `long`: 64-bit on Linux, 32-bit on wasm32. The Rust declarations
said `i64`, which had been harmless everywhere else and traps here. Fixed by
spelling it `libc::c_long`, which is right on every target.

**Two of everything is worse than a link error.** RUSTFLAGS carries
`--allow-multiple-definition` for the allocator shim, so a duplicate symbol is
not an error — it is a silent pick of whichever the linker saw first. MuPDF
bundles its own FreeType, HarfBuzz and zlib, and Tectonic has its own of each;
left alone, Tectonic's `hb_font_create` resolved to MuPDF's
`hb_face_set_user_data`, whose allocator wants a MuPDF context that does not
exist on that path, and the module trapped deep inside font setup.
`fetch-deps.sh` therefore builds MuPDF *against* Tectonic's copies. The one
namespace that cannot be shared, `pdf_*`, is renamed in the preprocessor —
`llvm-objcopy` has no `--redefine-syms` for wasm, which is how the other targets
do it.

**A build script that does not declare its inputs will not relink.** Rebuilding
MuPDF changed nothing until `preview_render`'s build script listed the archives
as `rerun-if-changed`; before that, cargo kept the previous link and the bug
above survived a "fix" that had definitely been applied.

## The libraries in `vendor/`

A browser has no system libraries, so the ones Tectonic normally finds with
pkg-config are compiled from source: **libpng** (for `\includegraphics` of PNGs,
via pdf_io), **FreeType** (font loading) and **Graphite2** (HarfBuzz wants it at
build time even though no mainstream document uses Graphite shaping). The bridge
crates' build scripts take this path automatically whenever the target family is
wasm; native builds are untouched.

**MuPDF** is fetched and built separately by `fetch-deps.sh --with-mupdf`,
because it is 145 MB of source and its terms are AGPL rather than the MIT the
rest of this tree uses. It is what turns the engine's output into the SVG the
preview shows, through `crates/preview_render`'s `Backend` trait. Building it
for wasi needed one source change — WASI has no `mkstemp`, patched in
`patches/mupdf-no-mkstemp.patch` — plus the symbol separation described above.

Two more are handled differently, because vendoring them is not sensible:

- **fontconfig** has nothing to find — a browser has no font directory. The
  crate supplies the API itself and reports that nothing matched, which is what
  a machine without the font installed would say. Fonts referenced by filename,
  as `fontspec`'s `Path=` option and `\font\f="[file.otf]"` do, still work.
- **ICU** would mean shipping ~30 MB of locale data. XeTeX uses thirteen of its
  functions, so those are reimplemented on `unicode-bidi` (the same UAX #9),
  `icu_segmenter` (ICU4X, the same project as ICU4C) and `encoding_rs`. See
  [`crates/bridge_icu/src/wasm_shim.rs`](../../crates/bridge_icu/src/wasm_shim.rs).
