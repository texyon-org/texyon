//! Downloading and unpacking source and toolchain archives.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;

const ATTEMPTS: usize = 3;

/// A `.tar.gz` holding one top-level directory.
pub struct Archive {
    /// What it is, for progress output.
    pub name: &'static str,
    /// Where to download it from.
    pub url: String,
}

impl Archive {
    /// Unpack the archive's top-level directory as `dest`, unless `dest` exists.
    ///
    /// `local` is a copy of the archive on disk, used instead of downloading.
    pub fn install(&self, dest: &Path, local: Option<&Path>) -> Result<()> {
        if dest.is_dir() {
            return Ok(());
        }
        let bytes = match local {
            Some(path) => {
                fs::read(path).with_context(|| format!("could not read {}", path.display()))?
            }
            None => {
                println!("fetching {} from {}", self.name, self.url);
                download(&self.url)?
            }
        };
        unpack_root(&bytes, dest).with_context(|| format!("could not unpack {}", self.name))
    }
}

fn download(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("texyon-xtask")
        .build()?;
    let mut last = None;
    for _ in 0..ATTEMPTS {
        match client
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
        {
            Ok(response) => return Ok(response.bytes()?.to_vec()),
            Err(error) => last = Some(error),
        }
    }
    Err(last.map_or_else(|| anyhow::anyhow!("no attempt made"), Into::into))
        .with_context(|| format!("could not download {url}"))
}

/// Unpack into a scratch directory beside `dest`, then move the single top-level directory into place,
/// so an interrupted run never leaves a half-filled `dest`.
fn unpack_root(bytes: &[u8], dest: &Path) -> Result<()> {
    let scratch = sibling(dest, "partial");
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }
    fs::create_dir_all(&scratch)?;
    tar::Archive::new(GzDecoder::new(bytes)).unpack(&scratch)?;

    let mut roots = fs::read_dir(&scratch)?.collect::<Result<Vec<_>, _>>()?;
    let [root] = roots.as_mut_slice() else {
        bail!("expected one top-level directory, found {}", roots.len());
    };
    fs::rename(root.path(), dest)?;
    fs::remove_dir_all(&scratch)?;
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}
