//! Profile-guided optimization, the same way for both targets: an instrumented
//! build replays the training sessions, and the release build is optimized
//! with what it counted.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::{
    cargo, corpus, native,
    pack::{self, Archive},
    process, scenario,
    session::Host,
    wasm,
};

/// The LLVM major whose raw profile format the minicov in crates/wasm/Cargo.toml writes.
const MINICOV_LLVM: u32 = 22;

/// Build the wasm module optimized with a profile of the training sessions.
pub fn wasm() -> Result<PathBuf> {
    let llvm = rustc_llvm_major()?;
    if llvm != MINICOV_LLVM {
        bail!(
            "rustc uses LLVM {llvm} but minicov writes LLVM {MINICOV_LLVM} profiles: \
             bump minicov in crates/wasm/Cargo.toml and MINICOV_LLVM here"
        );
    }
    let raw = raw_dir("wasm")?;
    let texyon = native::build(&cargo::host()?, &[], &native::Variant::Release)?;
    let pack = training_pack(&texyon, None)?;
    let module = wasm::build(&wasm::Variant::Instrumented)?;
    let profile = train(&Host::Wasm { module, pack }, &raw)?;
    wasm::build(&wasm::Variant::Optimized(profile))
}

/// Build the native binary for `target` optimized with a profile of the training sessions.
pub fn native(target: &str, features: &[String]) -> Result<PathBuf> {
    if target != cargo::host()? {
        bail!("training runs the instrumented binary, so build {target} on a machine of that kind");
    }
    if !features.iter().any(|feature| feature == "preview-mupdf") {
        bail!("training runs preview sessions, which need the preview-mupdf feature");
    }
    let raw = raw_dir(target)?;
    let binary = native::build(target, features, &native::Variant::Instrumented)?;
    // Its compiles of the training documents count too, as the command-line path.
    let pack = training_pack(&binary, Some(&raw))?;
    let profile = train(&Host::Native { binary, pack }, &raw)?;
    native::build(target, features, &native::Variant::Optimized(profile))
}

/// An empty directory for one target's raw counts.
fn raw_dir(name: &str) -> Result<PathBuf> {
    let raw = cargo::target_dir().join("pgo").join(name).join("raw");
    if raw.exists() {
        fs::remove_dir_all(&raw)?;
    }
    fs::create_dir_all(&raw)?;
    Ok(raw)
}

/// A pack with what the training documents read, so training never waits on the network.
fn training_pack(texyon: &Path, profile: Option<&Path>) -> Result<PathBuf> {
    let out = cargo::target_dir().join("pgo/pack");
    let mut seeds = pack::standard_seeds()?;
    seeds.extend(corpus::training_seeds()?);
    pack::build(texyon, profile, &seeds, &out, Archive::Skip)?;
    Ok(out)
}

/// Replay the session on every training document in `host`, and merge the counts in `raw`.
fn train(host: &Host, raw: &Path) -> Result<PathBuf> {
    for project in corpus::training()? {
        println!(
            "training on {}",
            project.root.join(&project.primary).display()
        );
        scenario::run(host, project, Some(raw))?;
    }
    let profile = raw.with_file_name("merged.profdata");
    process::run(
        Command::new(llvm_profdata()?)
            .arg("merge")
            .arg("-o")
            .arg(&profile)
            .arg(raw),
    )?;
    Ok(profile)
}

/// rustup's llvm-profdata, which reads the profile format of rustc's own LLVM.
fn llvm_profdata() -> Result<PathBuf> {
    let sysroot = process::stdout(Command::new("rustc").args(["--print", "sysroot"]))?;
    let name = if cfg!(windows) {
        "llvm-profdata.exe"
    } else {
        "llvm-profdata"
    };
    let tool = PathBuf::from(sysroot.trim())
        .join("lib/rustlib")
        .join(cargo::host()?)
        .join("bin")
        .join(name);
    if !tool.exists() {
        bail!("no llvm-profdata, run: rustup component add llvm-tools");
    }
    Ok(tool)
}

fn rustc_llvm_major() -> Result<u32> {
    let version = process::stdout(Command::new("rustc").arg("-vV"))?;
    version
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: "))
        .and_then(|version| version.split('.').next())
        .context("rustc -vV did not name its LLVM")?
        .parse()
        .context("rustc -vV named an LLVM version that is not a number")
}
