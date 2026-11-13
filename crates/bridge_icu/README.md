# The `texyon_bridge_icu` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It exposes the *C* API
of the [ICU4C] Unicode library the Rust/Cargo build framework, **with no Rust
bindings**.

[ICU4C]: https://unicode-org.github.io/icu/userguide/icu4c-readme.html

- [Main Git repository](https://github.com/texyon-org/texyon).

There are a variety of other low-level ICU-related crates available, with
[rust_icu](https://crates.io/crates/rust_icu) perhaps taking the most systematic
approach. This package is distinctive because:

- It uses TeXyoN’s dependency-finding framework, which supports both
  [pkg-config] and [vcpkg].
- It ensures the ICU C API is exposed to Cargo.
- Because it does not need to provide Rust bindings, it avoids a good deal of
  grief relating to bindgen, symbol versioning, etc.

[pkg-config]: https://www.freedesktop.org/wiki/Software/pkg-config/
[vcpkg]: https://vcpkg.readthedocs.io/

Ideally, though, one day this crate will be superseded by a true Rust "sys
crate".

If your project depends on this crate, Cargo will export for your build script
an environment variable named `DEP_ICUUC_INCLUDE_PATH`, which will be a
semicolon-separated list of C include directories enabling your code to include
the `unicode/*` headers.

You will need to ensure that your Rust code actually references this crate in
order for the linker to include linked libraries. A `use` statement will
suffice:

```rust
#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use texyon_bridge_icu;
```


## Cargo features

At the moment this crate does not provide any [Cargo features][features].

[features]: https://doc.rust-lang.org/cargo/reference/features.html
