//! `libpng` build script.
//!
//! On normal targets we find libpng on the system. WebAssembly has no system
//! libraries, so there we compile the vendored sources instead.

use std::{env, fs, path::PathBuf};
use texyon_dep_support::{Configuration, Dependency, Spec, target_is_wasm, wasm_vendor_dir};

struct LibpngSpec;

impl Spec for LibpngSpec {
    fn get_pkgconfig_spec(&self) -> &str {
        "libpng"
    }

    fn get_vcpkg_spec(&self) -> &[&str] {
        &["libpng"]
    }
}

/// libpng's core translation units. The ARM/Intel SIMD directories are left out
/// because wasm has neither.
const LIBPNG_SOURCES: &[&str] = &[
    "png.c",
    "pngerror.c",
    "pngget.c",
    "pngmem.c",
    "pngpread.c",
    "pngread.c",
    "pngrio.c",
    "pngrtran.c",
    "pngrutil.c",
    "pngset.c",
    "pngtrans.c",
    "pngwio.c",
    "pngwrite.c",
    "pngwtran.c",
    "pngwutil.c",
];

/// The headers a dependent crate needs in order to `#include <png.h>`.
const LIBPNG_HEADERS: &[&str] = &["png.h", "pngconf.h", "pnglibconf.h"];

fn build_vendored() {
    let src = wasm_vendor_dir("libpng");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // zlib comes from libz-sys, which is already in the tree by way of flate2.
    let zlib_include = env::var("DEP_Z_INCLUDE").expect("libz-sys did not export its include path");

    let mut cfg = cc::Build::new();
    cfg.warnings(false).include(&src).include(&zlib_include);

    for file in LIBPNG_SOURCES {
        cfg.file(src.join(file));
    }

    cfg.compile("png");

    // Present the headers the way a system install would.
    let include_dir = out_dir.join("include");
    fs::create_dir_all(&include_dir).expect("failed to create include dir");

    for header in LIBPNG_HEADERS {
        fs::copy(src.join(header), include_dir.join(header))
            .unwrap_or_else(|e| panic!("failed to copy {header}: {e}"));
    }

    println!("cargo:include-path={}", include_dir.display());
    println!("cargo:rerun-if-changed={}", src.display());
}

fn main() {
    if target_is_wasm() {
        build_vendored();
        return;
    }

    // Find any necessary deps.

    let dep_cfg = Configuration::default();
    let dep = Dependency::probe(LibpngSpec, &dep_cfg);

    // This is the key. What we print here will be propagated into depending
    // crates' build scripts as the environment variable DEP_LIBPNG_INCLUDE_PATH,
    // allowing them to find the headers internally. If/when we start vendoring
    // libPNG, this can become $OUT_DIR.
    let mut sep = "cargo:include-path=";

    dep.foreach_include_path(|p| {
        print!("{}{}", sep, p.to_str().unwrap());
        sep = ";";
    });

    println!();

    dep.emit();
}
