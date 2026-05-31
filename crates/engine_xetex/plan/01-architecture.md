# 01 — Target architecture

This file describes the Rust crate layout we're aiming for. Note that
the crate will pass through several intermediate layouts as the
translation proceeds; this document is the *destination*, not a
snapshot.

## Crate location

The work happens inside `crates/engine_xetex/`. We do **not** fork
the crate, create a parallel crate, or move files to a new path. The
existing name stays, the existing API surface stays, so downstream
crates (`tectonic`, etc.) need no churn.

## Current layout (session 0)

```
crates/engine_xetex/
├── Cargo.toml
├── build.rs                 # cc::Build for xetex/*.c
├── cbindgen.toml
├── plan/                    # ← this plan
├── src/
│   └── lib.rs               # thin FFI wrapper over tt_engine_xetex_main
└── xetex/
    ├── xetex-xetex0.c       # 18,694 LOC
    ├── xetex-ini.c          #  3,881 LOC (global state defs)
    ├── xetex-math.c         #  3,833 LOC
    ├── xetex-linebreak.c    #  2,843 LOC
    ├── xetex-shipout.c      #  2,451 LOC
    ├── xetex-ext.c          #  2,170 LOC
    ├── xetex-synctex.c      #  1,283 LOC
    ├── xetex-pagebuilder.c  #    804 LOC
    ├── xetex-output.c       #    768 LOC
    ├── xetex-pic.c          #    664 LOC
    ├── xetex-macos.c        #    664 LOC (conditional: is_mac_os only)
    ├── xetex-io.c           #    555 LOC
    ├── xetex-scaledmath.c   #    513 LOC
    ├── xetex-texmfmp.c      #    467 LOC
    ├── xetex-stringpool.c   #    206 LOC
    ├── xetex-errors.c       #    175 LOC
    ├── xetex-engine-interface.c  #  112 LOC
    ├── xetex-XeTeXOTMath.cpp     #  521 LOC, C++, not in scope
    ├── teckit-Engine.cpp         # 2,207 LOC, third-party, not in scope
    ├── teckit-NormalizationData.c  # 2,635 LOC, third-party, not in scope
    ├── xetex-xetexd.h       # shared declarations
    ├── xetex_format.h       # format-file layout
    ├── xetex-constants.h
    ├── xetex-core.h
    └── xetex-{ext,io,stringpool,synctex,swap,XeTeXOTMath}.h
```

## Target layout (end state)

```
crates/engine_xetex/
├── Cargo.toml                    # no build.rs (no C to compile)
├── src/
│   ├── lib.rs                    # public surface — unchanged API
│   ├── engine.rs                 # top-level run loop, main_control
│   ├── state/
│   │   ├── mod.rs                # EngineState struct
│   │   ├── mem.rs                # node arena (was mem[])
│   │   ├── eqtb.rs               # equivalents table
│   │   ├── save_stack.rs         # group save stack
│   │   ├── input_stack.rs        # tokenizer input stack
│   │   ├── string_pool.rs        # TeX string pool
│   │   ├── hash.rs               # control-sequence hash
│   │   └── fonts.rs              # font table
│   ├── node/
│   │   ├── mod.rs                # NodePtr<K> types & field accessor traits
│   │   ├── hlist.rs
│   │   ├── vlist.rs
│   │   ├── glue.rs
│   │   ├── penalty.rs
│   │   ├── char_node.rs
│   │   └── math.rs
│   ├── scaled_math.rs            # was xetex-scaledmath.c
│   ├── token.rs                  # token constants, cur_cmd/cur_chr
│   ├── scanner.rs                # scan_*, get_token, get_next
│   ├── expand.rs                 # expand, macro_call, conditionals
│   ├── main_control.rs           # the big dispatch loop
│   ├── pagebuilder.rs
│   ├── linebreak.rs              # K&P line breaker
│   ├── math.rs                   # math-mode layout
│   ├── shipout.rs                # DVI/XDV output
│   ├── synctex.rs
│   ├── errors.rs
│   ├── output.rs                 # print_*, show_*
│   ├── format_io.rs              # fmt dump / load
│   ├── fonts.rs                  # font loading glue (wraps xetex_layout)
│   ├── ext.rs                    # extension, \special, native fonts
│   ├── pic.rs                    # picture includes
│   └── io.rs                     # file I/O over bridge_core
└── plan/                         # still here for historical reference
```

Notes on this layout:

1.  **Modules reflect C files 1:1 where practical.** This makes the
    mapping between C origin and Rust destination trivial during
    translation, and `MODULE_STATUS.md` stays coherent.
2.  **State is pulled out of `lib.rs`** into its own `state/`
    submodule tree. During the migration, `state/mod.rs` will grow
    field-by-field from `globals.rs`.
3.  **Node accessors live under `node/`**, not scattered through
    `mem.rs`. `mem.rs` owns the arena; `node/` owns the *interpretation*
    of what's in it. Both borrow a `&[MemoryWord]` from the same
    backing store.

## Intermediate layout (during migration)

This is where we spend most of the life of the project:

```
crates/engine_xetex/
├── src/
│   ├── lib.rs                    # public FFI wrapper (unchanged early)
│   ├── globals.rs                # extern bindings to every C global
│   ├── ffi.rs                    # extern decls for remaining C functions
│   ├── mem.rs                    # growing collection of Rust accessors over the C mem[]
│   ├── scaled_math.rs            # early translations
│   ├── string_pool.rs            # early translations
│   └── ...                       # more as phases are completed
└── xetex/
    ├── xetex-xetex0.c            # shrinks over time
    ├── ...
```

Key mechanical invariants during migration:

-   `build.rs` still builds the remaining C files with `cc`.
-   Every Rust function called from C is `#[no_mangle] pub unsafe
    extern "C" fn <original_c_name>`. The C header keeps the same
    prototype, so call sites are unchanged and linking resolves to
    the Rust symbol.
-   Every C function called from Rust is declared in `src/ffi.rs`
    inside an `extern "C"` block. When the last caller of a given C
    function becomes Rust, we can drop the `extern "C"` wrapper from
    the Rust replacement (it becomes a regular safe Rust function).
-   Every C global used from Rust is declared in `src/globals.rs`
    with a `static mut` or via a getter/setter wrapping one. All
    accesses go through that module — no bare `extern` declarations
    scattered in translation files.

## How the FFI interleaving actually works

Concretely, consider `get_avail` (currently at `xetex-xetex0.c:170`):

### Step 1 — before translation

```c
// xetex-xetex0.c
int32_t get_avail(void) { /* ... body ... */ }
```

```c
// somewhere else in C:
int32_t p = get_avail();
```

### Step 2 — Rust translation

```rust
// src/mem.rs
// translated from xetex-xetex0.c:170 (get_avail)
#[no_mangle]
pub unsafe extern "C" fn get_avail() -> i32 {
    // ... direct translation, using globals::mem, globals::avail, etc
}
```

### Step 3 — C body removed

```c
// xetex-xetex0.c — body gone, declaration still lives in the header
// get_avail is now provided by Rust; the C side doesn't care.
```

### Step 4 — eventually, callers become Rust

Once every caller of `get_avail` is in Rust (or via `extern "C"`
the Rust side of the migration):

```rust
// src/mem.rs
pub fn get_avail(mem: &mut Mem) -> MemIdx { ... }

// The legacy extern "C" wrapper is kept only if some C caller remains;
// when the last C caller dies, the wrapper dies with it.
```

This three-step lifecycle per function is the core operation of this
project. It is designed so that:

-   At every step, the crate builds.
-   At every step, the full test suite runs.
-   Each step's blast radius is one function.

## Public API stability

`TexEngine`, `TexOutcome`, `FORMAT_SERIAL`, and the `c_api` module in
`lib.rs` are the public surface used by downstream tectonic crates. We
leave them alone until the translation is complete. At the very end,
we may deprecate the `c_api` module if it becomes pointless (since it
just calls into Rust-provided C symbols), but that is a post-migration
cleanup step, not part of this plan.

## What `build.rs` does during transition

`build.rs` currently uses `cc::Build` to compile every `.c` file in
`xetex/`. During the migration:

1.  Remove a `.c` file from the `cc` build only when *every* function
    it defined has been translated away.
2.  Until then, the file compiles, but specific function definitions
    inside it are deleted as they are ported. A file may contain only
    a few remaining functions for quite a while — that's fine.
3.  When the last `.c` file is removed, delete `build.rs` entirely and
    remove the `cc` build-dependency from `Cargo.toml`.
