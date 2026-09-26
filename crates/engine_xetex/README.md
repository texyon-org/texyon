# The `tectonic_engine_xetex` crate

[![](http://meritbadge.herokuapp.com/tectonic_engine_xetex)](https://crates.io/crates/tectonic_engine_xetex)

This crate is part of [the Tectonic
project](https://tectonic-typesetting.github.io/en-US/). It provides the
[XeTeX] engine as a reusable crate.

[XeTeX]: http://www.xetex.org/

- [API documentation](https://docs.rs/tectonic_engine_xetex/).
- [Main Git repository](https://github.com/tectonic-typesetting/tectonic/).


## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`external-harfbuzz`**: activates the same-named feature in
  the `tectonic_bridge_harfbuzz` dependency.


## Format introspection

The primitive catalog in `src/primitives.rs` follows the `tectonic_xetex_format`
introspection crate. This crate defines the format serial number that it
implements (in `tectonic_engine_xetex::FORMAT_SERIAL`) separately from the
maximum serial number supported by the introspection crate (in
`tectonic_xetex_format::LATEST_VERSION`).
