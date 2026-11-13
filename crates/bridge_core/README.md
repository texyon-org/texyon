# The `texyon_bridge_core` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It provides a C API to
core I/O services provided by the TeXyoN Rust code. This API is then consumed
by the various C/C++ "engines" implementing the TeX software.

- [Main Git repository](https://github.com/texyon-org/texyon).

If your project depends on this crate, Cargo will export for your build script
an environment variable named `DEP_TEXYON_BRIDGE_CORE_INCLUDE`, which will be
the name of a directory containing the generated `texyon_bridge_core.h` file
that exposes this crate’s C API.

You will need to ensure that your Rust code actually references this crate in
order for the linker to include the C API symbols. If you are not actively using
its Rust API, a `use` statement will suffice:

```rust
#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use texyon_bridge_core;
```


## Cargo features

This crate does not currently provide any [Cargo features][features].

[features]: https://doc.rust-lang.org/cargo/reference/features.html


## Updating the generated header

This crate exposes Rust functions to C/C++ code using a header file created by
[cbindgen]. To update the header, run:

[cbindgen]: https://github.com/eqrion/cbindgen/

```sh
cbindgen --output support/texyon_bridge_core_generated.h
```
