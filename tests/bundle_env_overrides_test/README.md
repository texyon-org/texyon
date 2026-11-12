# Bundle URL overrides Test

A test program for [`texyon_bundles::get_fallback_bundle_url`] under different compile-time environment variables.

`get_fallback_bundle_url` reads `$TEXYON_BUNDLE_LOCKED` through Rust's `option_env!` macro. The value is baked in at compile time, so each override scenario needs its own build of this program.

## Usage

```bash
# The built-in default bundle
cargo run

# A locked bundle
TEXYON_BUNDLE_LOCKED="https://example.com/bundle.tar" cargo run
```

The program prints the URL returned by `get_fallback_bundle_url()` to stdout. It is used by [`../bundle_env_overrides.rs`](../bundle_env_overrides.rs).
