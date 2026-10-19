# The `texyon_engine_xetex` crate

[![](http://meritbadge.herokuapp.com/texyon_engine_xetex)](https://crates.io/crates/texyon_engine_xetex)

This crate is part of [the Tectonic
project](https://tectonic-typesetting.github.io/en-US/). It provides the
[XeTeX] engine as a reusable crate.

[XeTeX]: http://www.xetex.org/

- [API documentation](https://docs.rs/texyon_engine_xetex/).
- [Main Git repository](https://github.com/tectonic-typesetting/tectonic/).
- [Architecture and validation](ARCHITECTURE.md).


## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`external-harfbuzz`**: activates the same-named feature in
  the `texyon_bridge_harfbuzz` dependency.


## Format introspection

The primitive catalog in `src/primitives.rs` follows the `texyon_xetex_format`
introspection crate. This crate defines the format serial number that it
implements (in `texyon_engine_xetex::FORMAT_SERIAL`) separately from the
maximum serial number supported by the introspection crate (in
`texyon_xetex_format::LATEST_VERSION`).
