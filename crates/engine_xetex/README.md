# The `texyon_engine_xetex` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It provides the
[XeTeX] engine as a reusable crate.

[XeTeX]: http://www.xetex.org/

- [Main Git repository](https://github.com/texyon-org/texyon).
- [Architecture and validation](ARCHITECTURE.md).


## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`external-harfbuzz`**: activates the same-named feature in
  the `texyon_bridge_harfbuzz` dependency.


## Format introspection

The primitive catalog in `src/tables/primitives.rs` follows the `texyon_xetex_format`
introspection crate. This crate defines the format serial number that it
implements (in `texyon_engine_xetex::FORMAT_SERIAL`) separately from the
maximum serial number supported by the introspection crate (in
`texyon_xetex_format::LATEST_VERSION`).
