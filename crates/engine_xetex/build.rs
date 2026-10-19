// Copyright 2021 the Tectonic Project
// Licensed under the MIT License.

//! XeTeX engine build script. Builds TECkit, the engine's only C++ code, and
//! finds the ICU version suffix for the Rust FFI.

use std::{env, fs, io::BufRead, path::PathBuf};

use texyon_cfg_support::*;

fn main() {
    let target = env::var("TARGET").unwrap();

    let core_include_dir = env::var("DEP_TEXYON_BRIDGE_CORE_INCLUDE").unwrap();
    let flate_include_dir = env::var("DEP_TEXYON_BRIDGE_FLATE_INCLUDE").unwrap();

    // Detect ICU version suffix (e.g. "_74") so Rust FFI can use #[link_name] with versioned names.
    //
    // A system ICU renames every symbol with its major version. The wasm build
    // has no system ICU and gets these entry points from Rust instead
    // (texyon_bridge_icu::wasm_shim), whose symbols are unsuffixed; that
    // bridge announces the suffix to use, so honour it when it is set.
    let icu_suffix = match env::var("DEP_ICUUC_VERSION_SUFFIX") {
        Ok(suffix) => suffix,
        Err(_) => {
            let icu_major = env::var("DEP_ICUUC_INCLUDE_PATH")
                .unwrap()
                .split(';')
                .find_map(|dir| {
                    let uvernum = PathBuf::from(dir).join("unicode/uvernum.h");
                    let f = fs::File::open(&uvernum).ok()?;
                    std::io::BufReader::new(f).lines().find_map(|line| {
                        let line = line.ok()?;
                        let line = line.trim();
                        if line.starts_with("#define U_ICU_VERSION_MAJOR_NUM") {
                            line.split_whitespace()
                                .nth(2)
                                .and_then(|v| v.parse::<u32>().ok())
                        } else {
                            None
                        }
                    })
                })
                .expect("could not detect ICU major version from uvernum.h");
            format!("_{icu_major}")
        }
    };
    println!("cargo:rustc-env=ICU_VERSION_SUFFIX={icu_suffix}");

    // If we want to profile, the default assumption is that we must force the
    // compiler to include frame pointers. We whitelist platforms that are
    // known to be able to profile *without* frame pointers: currently, only
    // Linux/x86_64.

    let profile_target_requires_frame_pointer: bool =
        target_cfg!(not(all(target_os = "linux", target_arch = "x86_64")));

    #[allow(unexpected_cfgs)]
    const PROFILE_BUILD_ENABLED: bool = cfg!(feature = "profile");

    let profile_config = |cfg: &mut cc::Build| {
        if PROFILE_BUILD_ENABLED {
            cfg.debug(true)
                .force_frame_pointer(profile_target_requires_frame_pointer);
        }
    };

    // Time to set up the C/C++ support libraries.

    let mut cxx_cfg = cc::Build::new();

    cxx_cfg.cpp(true);

    for flag in CXX_FLAGS {
        cxx_cfg.flag_if_supported(flag);
    }

    profile_config(&mut cxx_cfg);

    for p in &[".", &core_include_dir, &flate_include_dir] {
        cxx_cfg.include(p);
    }

    // Platform-specific adjustments:

    let is_big_endian = target_cfg!(target_endian = "big");
    if is_big_endian {
        cxx_cfg.define("WORDS_BIGENDIAN", "1");
    }

    if target.contains("-msvc") {
        cxx_cfg.flag("/EHsc");
    }

    // OK, back to generic build rules.

    for file in CXX_FILES {
        cxx_cfg.file(file);
    }

    cxx_cfg.compile("libtexyon_engine_xetex_cxx.a");

    // Rebuild if C/C++ files have changed. We scan the whole directory to get
    // the headers too.

    for file in PathBuf::from("xetex").read_dir().unwrap() {
        let file = file.unwrap();
        println!("cargo:rerun-if-changed={}", file.path().display());
    }
}

const CXX_FLAGS: &[&str] = &[
    "-std=c++14",
    "-Wall",
    "-Wdate-time",
    "-Wendif-labels",
    "-Wextra",
    "-Wformat=2",
    "-Wlogical-op",
    "-Wmissing-declarations",
    "-Wmissing-include-dirs",
    "-Wpointer-arith",
    "-Wredundant-decls",
    "-Wsuggest-attribute=noreturn",
    "-Wsuggest-attribute=format",
    "-Wshadow",
    "-Wswitch-bool",
    "-Wundef",
    // TODO: Fix existing warnings before enabling these:
    // "-Wdouble-promotion",
    // "-Wcast-align",
    // "-Wconversion",
    // "-Wmissing-variable-declarations",
    "-Wextra-semi",
    // "-Wsuggest-attribute=const",
    // "-Wsuggest-attribute=pure",
    // "-Wunreachable-code-aggresive",
    "-Wno-unused-parameter",
    "-Wno-implicit-fallthrough",
    "-fno-exceptions",
    "-fno-rtti",
];

const CXX_FILES: &[&str] = &["xetex/teckit-Engine.cpp"];
