# The `texyon_bundles` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It implements various
TeXyoN "bundles" that provide access to collections of TeX support files.

- [Main Git repository](https://github.com/texyon-org/texyon).


## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- `geturl-curl`: use the [curl] crate to implement HTTP requests. In order for
  this to take effect, you must use `--no-default-features` because
  `geturl-reqwest` is a default feature and it takes precedence
- `geturl-reqwest`: use the [reqwest] crate to implement HTTP requests (enabled
  by default)
- `native-tls-vendored`: if using [reqwest], activate the `vendored` option in
  the [native-tls] crate, causing OpenSSL to be vendored

[curl]: https://docs.rs/curl/
[reqwest]: https://docs.rs/reqwest/
[native-tls]: https://github.com/sfackler/rust-native-tls
