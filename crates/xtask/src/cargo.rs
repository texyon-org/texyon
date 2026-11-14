//! Cargo builds of the release artifacts.

use std::{env, path::PathBuf, process::Command};

use anyhow::Result;

use crate::{process, root};

/// A `cargo build` of one package for one target.
pub struct Build {
    /// The package.
    pub package: &'static str,
    /// The target triple.
    pub target: String,
    /// `false` for a debug build.
    pub release: bool,
    /// Cargo features to enable.
    pub features: Vec<String>,
    /// rustc flags for the target's crates, not for build scripts.
    pub rustflags: Vec<String>,
    /// Kept apart so instrumented and optimized builds do not rebuild each other.
    pub target_dir: PathBuf,
    /// Allow `-Z` flags on stable rustc.
    pub unstable: bool,
}

impl Build {
    /// A release build of `package` for `target` in the default target directory.
    pub fn release(package: &'static str, target: impl Into<String>) -> Self {
        Self {
            package,
            target: target.into(),
            release: true,
            features: Vec::new(),
            rustflags: Vec::new(),
            target_dir: target_dir(),
            unstable: false,
        }
    }

    /// The command, ready for more environment.
    pub fn command(&self) -> Command {
        let mut rustflags = self.rustflags.clone();
        // RUSTFLAGS would replace the target's flags outright, so fold it in.
        if let Ok(user) = env::var("RUSTFLAGS") {
            rustflags.push(user);
        }
        let variable = format!(
            "CARGO_TARGET_{}_RUSTFLAGS",
            self.target.to_uppercase().replace(['-', '.'], "_")
        );

        let mut cargo = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        cargo
            .current_dir(root())
            .args([
                "build",
                "--locked",
                "--package",
                self.package,
                "--target",
                &self.target,
            ])
            .arg("--target-dir")
            .arg(&self.target_dir)
            .env_remove("RUSTFLAGS")
            .env(variable, rustflags.join(" "));
        if self.release {
            cargo.arg("--release");
        }
        if !self.features.is_empty() {
            cargo.arg("--features").arg(self.features.join(","));
        }
        if self.unstable {
            cargo.env("RUSTC_BOOTSTRAP", "1");
        }
        cargo
    }

    /// Where the build puts the file `name`.
    pub fn artifact(&self, name: &str) -> PathBuf {
        let profile = if self.release { "release" } else { "debug" };
        self.target_dir.join(&self.target).join(profile).join(name)
    }
}

/// The workspace target directory.
pub fn target_dir() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(|| root().join("target"), PathBuf::from)
}

/// The triple of the machine running the build.
pub fn host() -> Result<String> {
    let version = process::stdout(Command::new("rustc").arg("-vV"))?;
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("rustc -vV did not name the host"))
}
