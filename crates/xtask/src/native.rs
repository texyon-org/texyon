//! The native `texyon` binary.

use std::path::PathBuf;

use anyhow::Result;

use crate::{
    cargo::{self, Build},
    deps, process,
};

/// How the binary is compiled.
pub enum Variant {
    /// Optimized without a profile.
    Release,
    /// Counting for profile-guided optimization, written where `LLVM_PROFILE_FILE` says.
    Instrumented,
    /// Optimized with the profile at this path.
    Optimized(PathBuf),
}

/// Build `texyon` for `target` with `features` and return the binary's path.
pub fn build(target: &str, features: &[String], variant: &Variant) -> Result<PathBuf> {
    let mut build = Build::release("texyon", target);
    build.features = features.to_vec();
    match variant {
        Variant::Release => {}
        Variant::Instrumented => {
            build.rustflags.push("-Cprofile-generate".into());
            build.target_dir = cargo::target_dir().join("pgo-instrumented");
        }
        Variant::Optimized(profile) => {
            build
                .rustflags
                .push(format!("-Cprofile-use={}", profile.display()));
        }
    }

    let mut cargo = build.command();
    let mupdf = deps::dir().join("mupdf");
    if mupdf.join("build/release/libmupdf.a").exists() {
        cargo.env("TEXYON_MUPDF_DIR", &mupdf);
    }
    process::run(&mut cargo)?;

    let name = if target.contains("windows") {
        "texyon.exe"
    } else {
        "texyon"
    };
    Ok(build.artifact(name))
}
