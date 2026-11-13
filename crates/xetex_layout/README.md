# The `texyon_xetex_layout` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It provides the font
loading and layout routines of [XeTeX] as a crate, with a Rust API.

[XeTeX]: http://xetex.sourceforge.net/

- [Main Git repository](https://github.com/texyon-org/texyon).

## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`external-harfbuzz`**: activates the same-named feature in
  the `texyon_bridge_harfbuzz` dependency.
