# The Tectonic Typesetting System

The `tectonic` crate delivers a modernized, complete, self-contained
[TeX](https://en.wikipedia.org/wiki/TeX)/[LaTeX](https://www.latex-project.org/)
engine, powered by [XeTeX](http://xetex.sourceforge.net/) and
[TeXLive](https://www.tug.org/texlive/). It provides a CLI and ties together the
functionality of a large collection of sub-crates.

- [User Website](https://tectonic-typesetting.github.io/)
- [The Tectonic Book](https://tectonic-typesetting.github.io/book/latest/)
- [Installation](https://tectonic-typesetting.github.io/book/latest/installation/) of the CLI
- [Detailed Build Instructions][build-inst]
- [API Documentation](https://docs.rs/tectonic/)
- [Issues](https://github.com/tectonic-typesetting/tectonic/issues/)
- [Changelog](https://github.com/tectonic-typesetting/tectonic/blob/release/CHANGELOG.md)

[build-inst]: https://tectonic-typesetting.github.io/book/latest/howto/build-tectonic/


## Building Tectonic

The Tectonic crates collectively depend on a relatively large number of
third-party libraries. See [the Book’s build instructions][build-inst] for a
listing and guidance on setting up the build.

The crate build can be customized with the following [Cargo features]:

[Cargo features]: https://doc.rust-lang.org/cargo/reference/features.html

- `external-harfbuzz`: build against an external version of the [Harfbuzz] text
  shaping library, rather than a vendored version (the default)
- `geturl-curl`: use the [curl] crate to implement HTTP requests. In order for
  this to take effect, you must use `--no-default-features` because
  `geturl-reqwest` is a default feature and it takes precedence
- `geturl-reqwest`: use the [reqwest] crate to implement HTTP requests (enabled
  by default)
- `native-tls-vendored`: if using [reqwest], activate the `vendored` option in
  the [native-tls] crate, causing OpenSSL to be vendored

[Harfbuzz]: https://harfbuzz.github.io/
[curl]: https://docs.rs/curl/
[reqwest]: https://docs.rs/reqwest/
[native-tls]: https://github.com/sfackler/rust-native-tls

[The Book][build-inst] describes some less-used features and provides a bit more
detail.


## Sub-crates

The main `tectonic` crate ties together the functionality of a number of
sub-crates:

- [`texyon_bridge_core`](https://crates.io/crates/texyon_bridge_core)
- [`texyon_bridge_flate`](https://crates.io/crates/texyon_bridge_flate)
- [`texyon_bridge_freetype2`](https://crates.io/crates/texyon_bridge_freetype2)
- [`texyon_bridge_graphite2`](https://crates.io/crates/texyon_bridge_graphite2)
- [`texyon_bridge_harfbuzz`](https://crates.io/crates/texyon_bridge_harfbuzz)
- [`texyon_bridge_icu`](https://crates.io/crates/texyon_bridge_icu)
- [`texyon_bundles`](https://crates.io/crates/texyon_bundles)
- [`texyon_cfg_support`](https://crates.io/crates/texyon_cfg_support)
- [`texyon_dep_support`](https://crates.io/crates/texyon_dep_support)
- [`texyon_docmodel`](https://crates.io/crates/texyon_docmodel)
- [`texyon_engine_bibtex`](https://crates.io/crates/texyon_engine_bibtex)
- [`texyon_engine_xdvipdfmx`](https://crates.io/crates/texyon_engine_xdvipdfmx)
- [`texyon_engine_xetex`](https://crates.io/crates/texyon_engine_xetex)
- [`texyon_errors`](https://crates.io/crates/texyon_errors)
- [`texyon_geturl`](https://crates.io/crates/texyon_geturl)
- [`texyon_io_base`](https://crates.io/crates/texyon_io_base)
- [`texyon_pdf_io`](https://crates.io/crates/texyon_pdf_io)
- [`texyon_status_base`](https://crates.io/crates/texyon_status_base)
- [`texyon_xdv`](https://crates.io/crates/texyon_xdv)
- [`texyon_xetex_layout`](https://crates.io/crates/texyon_xetex_layout)
