// Copyright 2016-2026 the Tectonic Project
// Licensed under the MIT License.

//! I/O providers that exist to make tests and fixtures easy to assemble.
//!
//! These are shipped rather than `#[cfg(test)]` because the things that need
//! them — integration tests, differential-test fixtures, the live-preview
//! fixture support path — all live outside this crate.

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use tectonic_status_base::StatusBackend;

use crate::{InputHandle, InputOrigin, IoProvider, OpenResult, OutputHandle};

/// An [`IoProvider`] that serves exactly one file from the filesystem.
///
/// The file answers to its own base name and nothing else, which is what makes
/// it useful for handing an engine a specific format dump without putting the
/// whole directory it lives in on the search path.
pub struct SingleInputFileIo {
    name: String,
    full_path: PathBuf,
}

impl SingleInputFileIo {
    /// Serve the file at `path` under its base name.
    ///
    /// # Panics
    ///
    /// Panics if the path has no base name, or if that name is not UTF-8.
    pub fn new(path: &Path) -> SingleInputFileIo {
        let p = path.to_path_buf();

        SingleInputFileIo {
            name: p.file_name().unwrap().to_str().unwrap().to_owned(),
            full_path: p,
        }
    }
}

impl IoProvider for SingleInputFileIo {
    fn output_open_name(&mut self, _: &str) -> OpenResult<OutputHandle> {
        OpenResult::NotAvailable
    }

    fn output_open_stdout(&mut self) -> OpenResult<OutputHandle> {
        OpenResult::NotAvailable
    }

    fn input_open_name(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        if name == self.name {
            OpenResult::Ok(InputHandle::new(
                name,
                File::open(&self.full_path).unwrap(),
                InputOrigin::Filesystem,
            ))
        } else {
            OpenResult::NotAvailable
        }
    }
}
