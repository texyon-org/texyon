# The `texyon_dep_support` crate

This crate is part of [TeXyoN](https://github.com/texyon-org/texyon), which grew
out of [Tectonic](https://tectonic-typesetting.github.io/). It provides build-time
utilities for finding external library dependencies, allowing either
[pkg-config] or [vcpkg] to be used as the dep-finding backend, and providing
whatever fiddly features are needed to enable the TeXyoN build process.

- [Main Git repository](https://github.com/texyon-org/texyon).

[pkg-config]: https://www.freedesktop.org/wiki/Software/pkg-config/
[vcpkg]: https://vcpkg.readthedocs.io/
