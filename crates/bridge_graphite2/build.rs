// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

//! graphite2 build script. On normal targets we find it externally. WebAssembly
//! has no system libraries, so there we compile the vendored sources.

use std::{env, fs, path::PathBuf};
use tectonic_dep_support::{target_is_wasm, wasm_vendor_dir, Configuration, Dependency, Spec};

struct Graphite2Spec;

impl Spec for Graphite2Spec {
    fn get_pkgconfig_spec(&self) -> &str {
        "graphite2"
    }

    fn get_vcpkg_spec(&self) -> &[&str] {
        &["graphite2"]
    }
}

/// Graphite2's translation units, mirroring `src/CMakeLists.txt`.
///
/// `call_machine.cpp` is the portable interpreter; `direct_machine.cpp` is the
/// computed-goto variant, which does not build for wasm. `FileFace.cpp` is
/// omitted along with `GRAPHITE2_NFILEFACE` since fonts arrive as bytes, and
/// `json.cpp` along with `GRAPHITE2_NTRACING`.
const GRAPHITE2_SOURCES: &[&str] = &[
    "call_machine.cpp",
    "gr_char_info.cpp",
    "gr_features.cpp",
    "gr_face.cpp",
    "gr_font.cpp",
    "gr_logging.cpp",
    "gr_segment.cpp",
    "gr_slot.cpp",
    "CmapCache.cpp",
    "Code.cpp",
    "Collider.cpp",
    "Decompressor.cpp",
    "Face.cpp",
    "FeatureMap.cpp",
    "Font.cpp",
    "GlyphFace.cpp",
    "GlyphCache.cpp",
    "Intervals.cpp",
    "Justifier.cpp",
    "NameTable.cpp",
    "Pass.cpp",
    "Position.cpp",
    "Segment.cpp",
    "Silf.cpp",
    "Slot.cpp",
    "Sparse.cpp",
    "TtfUtil.cpp",
    "UtfCodec.cpp",
];

fn build_vendored() {
    let src = wasm_vendor_dir("graphite2");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut cfg = cc::Build::new();
    cfg.cpp(true)
        .warnings(false)
        .flag("-std=c++11")
        .include(src.join("include"))
        .include(src.join("src"))
        .include(src.join("src/inc"))
        .define("GRAPHITE2_STATIC", "1")
        .define("GRAPHITE2_NTRACING", "1")
        .define("GRAPHITE2_NFILEFACE", "1");

    for file in GRAPHITE2_SOURCES {
        cfg.file(src.join("src").join(file));
    }

    cfg.compile("graphite2");

    // Present the headers the way a system install would: consumers include
    // <graphite2/Font.h>, so the directory above `graphite2/` is what goes on
    // the include path.
    let include_dir = out_dir.join("include");
    let dest = include_dir.join("graphite2");
    fs::create_dir_all(&dest).expect("failed to create include dir");

    for entry in fs::read_dir(src.join("include/graphite2")).expect("failed to read header dir") {
        let entry = entry.expect("failed to read dir entry");
        fs::copy(entry.path(), dest.join(entry.file_name())).expect("failed to copy header");
    }

    println!("cargo:include-path={}", include_dir.display());
    println!("cargo:define_static=1");
    println!("cargo:rerun-if-changed={}", src.display());
}

fn main() {
    if target_is_wasm() {
        build_vendored();
        return;
    }

    let cfg = Configuration::default();
    let dep = Dependency::probe(Graphite2Spec, &cfg);

    // This is the key. What we print here will be propagated into depending
    // crates' build scripts as the environment variable
    // DEP_GRAPHITE2_INCLUDE_PATH, allowing them to find the headers internally.
    // If/when we start vendoring graphite2, this can become $OUT_DIR.

    let mut sep = "cargo:include-path=";

    dep.foreach_include_path(|p| {
        print!("{}{}", sep, p.to_str().unwrap());
        sep = ";";
    });

    println!();

    dep.emit();

    // As a special case, code that compiles against graphite2 must also
    // sometimes provide -DGRAPHITE2_STATIC. We'd prefer not to get into the
    // business of propagating arbitrary cflags through our build system, so we
    // indicate it with a specialized variable.

    let target = env::var("TARGET").unwrap();

    let rustflags = env::var("CARGO_ENCODED_RUSTFLAGS")
        .unwrap_or_else(|_| env::var("RUSTFLAGS").unwrap_or_default());

    let define_static_flag = if target.contains("-msvc") && rustflags.contains("+crt-static") {
        "1"
    } else {
        ""
    };

    println!("cargo:define_static={define_static_flag}");
}
