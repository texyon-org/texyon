//! Fixtures shared by this crate's unit tests.

use std::path::{Path, PathBuf};

use tectonic_bundles::{dir::DirBundle, Bundle};
use tectonic_errors::Result;
use tectonic_io_base::{digest::DigestData, InputHandle, IoProvider, OpenResult};
use tectonic_status_base::StatusBackend;

/// Path to a file in the workspace's shared `tests/` tree.
pub fn asset_path(parts: &[&str]) -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    path.extend(parts);
    path
}

/// The shared `tests/assets` directory, presented as a support bundle.
pub struct AssetBundle(DirBundle);

impl Default for AssetBundle {
    fn default() -> Self {
        AssetBundle(DirBundle::new(asset_path(&["assets"])))
    }
}

impl IoProvider for AssetBundle {
    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        self.0.input_open_name(name, status)
    }
}

impl Bundle for AssetBundle {
    fn get_digest(&mut self) -> Result<DigestData> {
        Ok(DigestData::zeros())
    }

    fn all_files(&self) -> Vec<String> {
        self.0.all_files()
    }
}
