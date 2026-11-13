# The `texyon_geturl` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It provides an
interface for fetching URLs using one of several HTTP backends.

- [Main Git repository](https://github.com/texyon-org/texyon).


## Cargo features

This crate provides the following [Cargo features][features]:

[features]: https://doc.rust-lang.org/cargo/reference/features.html

- **`curl`**: use the [curl] crate as a backend for performing URL gets.
- **`reqwest`** (enabled by default): use the [reqwest] crate as a backend for
  performing URL gets.

[curl]: https://docs.rs/curl/
[reqwest]: https://docs.rs/reqwest/

There is always a "null" backend available, which will always return errors. If
more than one backend is enabled, their prioritization is:

- `reqwest` (most preferred)
- `curl`
- `null` (least preferred)
