//! Integration tests of the bundles crate.

use std::env;
use std::path::PathBuf;
use std::process::Command;

const DEFAULT_BUNDLE_URL: &str = "https://bundles.texyon.org/texlive2026.ttb";
const TEST_BUNDLE_LOCKED: &str = "https://example.com/locked_bundle.tar";

/// Runs the test project and gets the output
fn run_test_program(env_vars: &[(&str, &str)]) -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let test_dir = PathBuf::from(&manifest_dir)
        .join("tests")
        .join("bundle_env_overrides_test");
    let target_platform = env::var("TARGET").unwrap();

    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--quiet")
        .arg("--target")
        .arg(target_platform)
        .current_dir(&test_dir)
        .env_remove("TEXYON_BUNDLE_LOCKED");

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output().expect("Failed to run test program");
    assert!(
        output.status.success(),
        "Test program failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn test_bundle_default() {
    assert_eq!(run_test_program(&[]), DEFAULT_BUNDLE_URL);
}

#[test]
fn test_bundle_locked() {
    assert_eq!(
        run_test_program(&[("TEXYON_BUNDLE_LOCKED", TEST_BUNDLE_LOCKED)]),
        TEST_BUNDLE_LOCKED
    );
}

#[test]
fn test_bundle_locked_empty_is_ignored() {
    assert_eq!(
        run_test_program(&[("TEXYON_BUNDLE_LOCKED", "")]),
        DEFAULT_BUNDLE_URL
    );
}
