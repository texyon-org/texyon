//! LaTeX golden-file regression tests for the engine_xetex Rustification.
//!
//! Each test compiles a LaTeX document through the full Tectonic driver (with
//! the real web bundle) and compares the SHA-256 of the XDV output against a
//! stored hash.  A change in any hash means a regression in an already-passing
//! document.
//!
//! # Requirements
//!
//! These tests need the Tectonic bundle to be locally cached.  If it is not
//! available the tests are **skipped** (not failed).  Run any normal tectonic
//! build first:
//!
//! ```text
//! cargo run -p tectonic -- tests/latex_docs/align_table.tex
//! ```
//!
//! # Blessing (updating goldens)
//!
//! ```text
//! TECTONIC_BLESS=1 cargo test -p tectonic --test latex_goldens
//! ```

use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "util/mod.rs"]
mod util;

/// Directory holding the LaTeX source documents.
fn docs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("latex_docs")
}

/// Directory where hash files are stored.
fn hashes_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("latex_docs")
        .join("goldens")
}

fn bless_mode() -> bool {
    env::var("TECTONIC_BLESS").map(|v| v == "1").unwrap_or(false)
}

/// Return the path to the `tectonic` binary built by Cargo.
fn tectonic_bin() -> PathBuf {
    util::cargo_dir()
        .join("tectonic")
        .with_extension(env::consts::EXE_EXTENSION)
}

/// Compile a LaTeX document and return the SHA-256 hex of its XDV, or
/// `None` if the bundle is not cached (test should be skipped).
fn compile_and_hash(tex_path: &Path) -> Option<String> {
    let tempdir = tempfile::Builder::new()
        .prefix("tectonic_latex_golden")
        .tempdir()
        .expect("creating tempdir");

    let bin = tectonic_bin();
    if !bin.exists() {
        // Binary not built yet — skip.
        return None;
    }

    let output = Command::new(&bin)
        .arg("--outfmt=xdv")
        .arg("--outdir")
        .arg(tempdir.path())
        .arg(tex_path)
        .output()
        .expect("spawning tectonic");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If the failure is "bundle not available / offline", skip.
        if stderr.contains("not available")
            || stderr.contains("offline")
            || stderr.contains("connection")
            || stderr.contains("network")
        {
            return None;
        }
        panic!(
            "tectonic failed on {}:\nstdout: {}\nstderr: {}",
            tex_path.display(),
            String::from_utf8_lossy(&output.stdout),
            stderr,
        );
    }

    // Find the XDV output.
    let stem = tex_path.file_stem().unwrap().to_str().unwrap();
    let xdv_path = tempdir.path().join(format!("{stem}.xdv"));

    if !xdv_path.exists() {
        panic!("no XDV produced for {}", tex_path.display());
    }

    let data = fs::read(&xdv_path).unwrap();
    let hash = format!("{:x}", Sha256::digest(&data));
    Some(hash)
}

fn run_case(stem: &str) {
    let tex_path = docs_dir().join(format!("{stem}.tex"));
    let hash_path = hashes_dir().join(format!("{stem}.sha256"));

    let actual_hash = match compile_and_hash(&tex_path) {
        Some(h) => h,
        None => {
            eprintln!("SKIP {stem}: bundle not available");
            return;
        }
    };

    if bless_mode() {
        fs::create_dir_all(hashes_dir()).expect("creating goldens dir");
        fs::write(&hash_path, &actual_hash).unwrap_or_else(|e| {
            panic!("writing {hash_path:?}: {e}");
        });
        eprintln!("  blessed {stem} → {actual_hash}");
    } else {
        match fs::read_to_string(&hash_path) {
            Ok(expected) => {
                let expected = expected.trim();
                if actual_hash != expected {
                    panic!(
                        "{stem}: XDV hash mismatch!\n  expected: {expected}\n  actual:   {actual_hash}\n  \
                         Run with TECTONIC_BLESS=1 to update the golden."
                    );
                }
            }
            Err(_) => {
                eprintln!("SKIP {stem}: no golden hash yet. Run with TECTONIC_BLESS=1 to generate.");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// LaTeX tabular environments (exercises \halign via LaTeX).
#[test]
fn align_table() {
    run_case("align_table");
}

/// Display and inline math, align*, cases, matrix (exercises math mode).
#[test]
fn math_modes() {
    run_case("math_modes");
}

/// LaTeX conditionals: \newif, \ifnum, \@ifstar, nested ifs.
#[test]
fn conditionals() {
    run_case("conditionals");
}

/// Italic text and \/correction (exercises append_italic_correction fix).
#[test]
fn fonts_italic() {
    run_case("fonts_italic");
}
