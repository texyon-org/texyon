//! The support pack: the format dump and the support files a session reads
//! before the bundle, listed by `manifest.json` beside them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The manifest's file name.
pub const MANIFEST: &str = "manifest.json";

/// `manifest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    /// Layout version.
    pub version: u32,
    /// The format dump's file name.
    pub format: String,
    /// The bundle the files came from, as its hex digest.
    pub bundle_digest: String,
    /// The engine format serial the dump was made with.
    pub format_serial: u32,
    /// The size of every file together.
    pub total_bytes: u64,
    /// Every file and its size.
    pub files: BTreeMap<String, u64>,
}
