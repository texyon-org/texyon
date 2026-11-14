// Copyright 2016-2020 the Tectonic Project
// Licensed under the MIT License.

//! This inelegant module helps with testing.
//!
//! We would like it if all of our tests -- unit tests (embedded in this
//! crate), integration tests (in `tests/`), and doctests -- used the "test
//! bundle" in `tests/assets/`, rather than having to touch the network to
//! work. Unfortunately, the integration and doctests are compiled against the
//! "default" build of the main crate -- that is, the version built *without*
//! #[cfg(test)]. Therefore, to support such usage in all three scenarios, we
//! need to build the code into the main crate unconditionally. We can hide it
//! from the documentation, but we can't use #[cfg(test)] attributes to
//! conditionally compile code.
//!
//! So, that's what we do. Importantly, the code here should all be nice and
//! lightweight, and have very low impact on the final binary artifacts we
//! produce. We also take care not to embed the build directory into the
//! final binaries.
//!
//! There are two main functionalities covered in this module. First, there
//! are some functions that help in locating test assets and making a fakey
//! "bundle" that uses them.
//!
//! Second, there is some infrastructure to activate a "test" mode that
//! affects the crate's behavior in a few ways. The binary we distribute
//! activates this mode when a magic environment variable is set.
//!
//! This is all implemented so that out doctests can look totally innocuous --
//! they are what you'd actually want in your own code, except we have a hidden
//! one-liner when needed:
//!
//! # texyon::test_util::activate_test_mode_augmented(env!("CARGO_MANIFEST_DIR"));
//!
//! That call simultaneously tells this module where to find the test assets,
//! and also activates the test mode.

use std::{env, ffi::OsStr, path::PathBuf, sync::OnceLock};
use texyon_bundles::{Bundle, dir::DirBundle};
use texyon_errors::Result;

use crate::{
    digest::DigestData,
    io::{InputHandle, IoProvider, OpenResult},
    status::StatusBackend,
};

/// The name of the environment variable that tells the `texyon` binary where
/// to find the testing resource files.
pub const TEST_ROOT_ENV_VAR: &str = "TEXYON_INTERNAL_TEST_ROOT";

/// The test root set in this process, which tests share without touching the
/// environment.
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Set this process's test root to the specified directory with a path
/// element "tests" appended, so that `test_path()` and the other pieces of
/// testing infrastructure in this module start working. The first call wins.
///
/// The peculiar form of this function makes for easy one-liners in the test
/// code exploiting the environment variable $CARGO_MANIFEST_DIR.
pub fn set_test_root_augmented<V: AsRef<OsStr>>(root: V) {
    TEST_ROOT.get_or_init(|| PathBuf::from(root.as_ref()).join("tests"));
}

/// The test root set in this process, or else the one the `texyon` binary was
/// launched with.
fn test_root() -> Option<PathBuf> {
    TEST_ROOT
        .get()
        .cloned()
        .or_else(|| env::var_os(TEST_ROOT_ENV_VAR).map(PathBuf::from))
}

/// Activate this crate's "test mode", if-and-only-if a test root is known.
/// This allows testing of the Tectonic executable in a transparent way —
/// notably, we avoid embedding the build prefix in the resulting binary, and
/// we don't need to pass it any magical command line arguments.
pub fn maybe_activate_test_mode() {
    if test_root().is_none() {
        return;
    }

    crate::config::activate_config_test_mode(true);
}

/// A combination of the two above functions. Set the "test root" variable,
/// making it such that the testing infrastructure in this module can work;
/// and then activate "test mode", such that certain other parts of the crate
/// alter their behavior for use in the test environment. This makes for
/// convenient doctests — you can secretly run them in the special test setup
/// with a one-liner:
///
/// # texyon::test_util::activate_test_mode_augmented(env!("CARGO_MANIFEST_DIR"));
pub fn activate_test_mode_augmented<V: AsRef<OsStr>>(root: V) {
    set_test_root_augmented(root);
    maybe_activate_test_mode();
}

/// Obtain a path to a testing resource file. A test root must have been set
/// with `set_test_root_augmented`, or passed in `TEST_ROOT_ENV_VAR`.
pub fn test_path(parts: &[&str]) -> PathBuf {
    let mut path = test_root().expect(
        "Tectonic testing infrastructure cannot be used without \
         setting the test root",
    );
    path.push(parts.iter().collect::<PathBuf>());
    path
}

/// Utility for being able to treat the "assets/" directory as a bundle.
///
/// I think we want to always wrap DirBundle so that we can override
/// `get_digest()`? But once DirBundle implements `get_digest()` for real we
/// could consider just dropping this type altogether.
pub struct TestBundle(DirBundle);

impl Default for TestBundle {
    fn default() -> Self {
        TestBundle(DirBundle::new(test_path(&["assets"])))
    }
}

impl IoProvider for TestBundle {
    // All other functions can default to NotAvailable/error.
    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        self.0.input_open_name(name, status)
    }
}

impl Bundle for TestBundle {
    fn get_digest(&mut self) -> Result<DigestData> {
        Ok(DigestData::zeros())
    }

    fn all_files(&self) -> Vec<String> {
        self.0.all_files()
    }
}
