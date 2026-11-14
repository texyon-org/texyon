//! A prebuilt support pack in front of the bundle.

use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    thread::{self, JoinHandle},
};

use serde::Deserialize;
use texyon_bundles::Bundle;
use texyon_errors::{Result, anyhow::anyhow};
use texyon_io_base::{
    InputHandle, InputOrigin, IoProvider, OpenResult, digest::DigestData, format_cache::FormatCache,
};
use texyon_status_base::{NoopStatusBackend, StatusBackend};

/// The manifest `tools/wasm/build-assets.sh` writes next to the pack's files.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: String,
    files: BTreeMap<String, u64>,
    bundle_digest: String,
    format_serial: u32,
}

/// Why a support pack could not be used.
#[derive(Debug)]
pub enum PackError {
    /// A pack file could not be read.
    Read {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        error: io::Error,
    },
    /// The manifest is not one this build understands.
    Manifest(serde_json::Error),
    /// The manifest names a file outside the pack directory.
    Name(String),
    /// The manifest's bundle digest does not parse.
    Digest(String),
    /// The format dump could not be written to the format cache.
    SeedFormat(texyon_errors::Error),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => write!(f, "could not read {}: {error}", path.display()),
            Self::Manifest(error) => write!(f, "invalid support pack manifest: {error}"),
            Self::Name(name) => write!(f, "support pack names a file outside the pack: {name}"),
            Self::Digest(digest) => write!(f, "invalid bundle digest in support pack: {digest}"),
            Self::SeedFormat(error) => {
                write!(f, "could not cache the support pack's format: {error}")
            }
        }
    }
}

impl std::error::Error for PackError {}

/// Support files and a format dump compiled from one bundle, read by name.
#[derive(Debug)]
pub struct SupportPack {
    files_dir: PathBuf,
    files: BTreeMap<String, u64>,
    format: String,
    bundle_digest: DigestData,
    format_serial: u32,
}

impl SupportPack {
    /// Read the pack whose `manifest.json` and `files/` are in `dir`.
    pub fn open(dir: &Path) -> Result<Self, PackError> {
        let path = dir.join("manifest.json");
        let text = fs::read_to_string(&path).map_err(|error| PackError::Read { path, error })?;
        let manifest: Manifest = serde_json::from_str(&text).map_err(PackError::Manifest)?;
        if let Some(name) = std::iter::once(&manifest.format)
            .chain(manifest.files.keys())
            .find(|name| !is_plain_name(name))
        {
            return Err(PackError::Name(name.clone()));
        }
        let bundle_digest = DigestData::from_str(&manifest.bundle_digest)
            .map_err(|_| PackError::Digest(manifest.bundle_digest.clone()))?;
        Ok(Self {
            files_dir: dir.join("files"),
            files: manifest.files,
            format: manifest.format,
            bundle_digest,
            format_serial: manifest.format_serial,
        })
    }

    /// Digest of the bundle the pack was compiled from.
    pub fn bundle_digest(&self) -> DigestData {
        self.bundle_digest
    }

    /// Put the pack's format dump in `cache` if it is the one asked for and this engine can load it.
    ///
    /// `cache` must be keyed by [`Self::bundle_digest`]. Anything else is left
    /// for the caller to generate.
    pub fn seed_format(
        &self,
        cache: &mut FormatCache,
        name: &str,
        serial: u32,
    ) -> Result<(), PackError> {
        if name != self.format || serial != self.format_serial {
            return Ok(());
        }
        let mut status = NoopStatusBackend::default();
        if let OpenResult::Ok(_) = cache.input_open_format(name, &mut status) {
            return Ok(());
        }
        let path = self.files_dir.join(&self.format);
        let bytes = fs::read(&path).map_err(|error| PackError::Read { path, error })?;
        cache
            .write_format(name, &bytes, &mut status)
            .map_err(PackError::SeedFormat)
    }

    fn open_name(&self, name: &str) -> OpenResult<InputHandle> {
        if !self.files.contains_key(name) {
            return OpenResult::NotAvailable;
        }
        match File::open(self.files_dir.join(name)) {
            Ok(file) => OpenResult::Ok(InputHandle::new_read_only(
                name,
                BufReader::new(file),
                InputOrigin::Other,
            )),
            Err(error) => OpenResult::Err(error.into()),
        }
    }
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

/// Opens the bundle a pack was compiled from.
pub type BundleOpener = Arc<dyn Fn() -> Result<Box<dyn Bundle>> + Send + Sync>;

/// A bundle that answers from a [`SupportPack`] first and opens the real one on the first miss.
///
/// The real bundle's index starts downloading in the background as soon as
/// this is built, so a miss rarely waits for all of it.
pub struct PackedBundle {
    pack: SupportPack,
    open: BundleOpener,
    warming: Option<JoinHandle<()>>,
    bundle: Option<Box<dyn Bundle>>,
}

/// A name no bundle holds, so looking it up loads the index and fetches nothing.
const INDEX_PROBE: &str = "texyon-index-probe.invalid";

impl PackedBundle {
    /// Serve `pack` in front of the bundle `open` returns.
    pub fn new(pack: SupportPack, open: BundleOpener) -> Self {
        let warm = Arc::clone(&open);
        let warming = thread::Builder::new()
            .name("texyon-bundle-index".into())
            .spawn(move || {
                // Failures resurface on the real open, where they can be reported.
                if let Ok(mut bundle) = warm() {
                    let _ = bundle.input_open_name(INDEX_PROBE, &mut NoopStatusBackend::default());
                }
            })
            .ok();
        Self {
            pack,
            open,
            warming,
            bundle: None,
        }
    }

    fn bundle(&mut self) -> Result<&mut Box<dyn Bundle>> {
        let bundle = match self.bundle.take() {
            Some(bundle) => bundle,
            None => {
                // Both caches write the index under one name, so never race the warmer.
                if let Some(warming) = self.warming.take() {
                    let _ = warming.join();
                }
                let mut bundle = (self.open)()?;
                let digest = bundle.get_digest()?;
                if digest != self.pack.bundle_digest {
                    return Err(anyhow!(
                        "the support pack was built for bundle {}, but the bundle is now {digest}",
                        self.pack.bundle_digest
                    ));
                }
                bundle
            }
        };
        Ok(self.bundle.insert(bundle))
    }
}

impl IoProvider for PackedBundle {
    fn input_open_name(
        &mut self,
        name: &str,
        status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        match self.pack.open_name(name) {
            OpenResult::NotAvailable => match self.bundle() {
                Ok(bundle) => bundle.input_open_name(name, status),
                Err(error) => OpenResult::Err(error),
            },
            found => found,
        }
    }
}

impl Bundle for PackedBundle {
    fn get_digest(&mut self) -> Result<DigestData> {
        Ok(self.pack.bundle_digest)
    }

    fn all_files(&self) -> Vec<String> {
        match &self.bundle {
            Some(bundle) => bundle.all_files(),
            None => self.pack.files.keys().cloned().collect(),
        }
    }
}

impl fmt::Debug for PackedBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackedBundle")
            .field("pack", &self.pack)
            .field("bundle_open", &self.bundle.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Read},
        sync::Mutex,
    };

    use super::*;

    const PACK_DIGEST: &str = "09b7ed377a439be5d08cca82cbd0df0083635e9e2206d8ad9ba51fd1de20b5c5";
    const OTHER_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const SERIAL: u32 = 35;

    fn write_pack(dir: &Path, files: &[(&str, &str)]) {
        fs::create_dir_all(dir.join("files")).unwrap();
        let mut sizes = BTreeMap::new();
        for (name, text) in files {
            fs::write(dir.join("files").join(name), text).unwrap();
            sizes.insert(*name, text.len());
        }
        let manifest = serde_json::json!({
            "version": 1,
            "format": "latex.fmt",
            "bundleDigest": PACK_DIGEST,
            "formatSerial": SERIAL,
            "totalBytes": 0,
            "files": sizes,
        });
        fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    }

    /// A bundle that records every name it is asked for.
    struct FakeBundle {
        digest: DigestData,
        files: BTreeMap<String, String>,
        lookups: Arc<Mutex<Vec<String>>>,
    }

    impl IoProvider for FakeBundle {
        fn input_open_name(
            &mut self,
            name: &str,
            _status: &mut dyn StatusBackend,
        ) -> OpenResult<InputHandle> {
            self.lookups.lock().unwrap().push(name.to_owned());
            match self.files.get(name) {
                Some(text) => OpenResult::Ok(InputHandle::new_read_only(
                    name,
                    Cursor::new(text.clone().into_bytes()),
                    InputOrigin::Other,
                )),
                None => OpenResult::NotAvailable,
            }
        }
    }

    impl Bundle for FakeBundle {
        fn get_digest(&mut self) -> Result<DigestData> {
            Ok(self.digest)
        }

        fn all_files(&self) -> Vec<String> {
            self.files.keys().cloned().collect()
        }
    }

    fn packed(
        dir: &Path,
        digest: &str,
        files: &[(&str, &str)],
    ) -> (PackedBundle, Arc<Mutex<Vec<String>>>) {
        let pack = SupportPack::open(dir).unwrap();
        let lookups = Arc::new(Mutex::new(Vec::new()));
        let digest = DigestData::from_str(digest).unwrap();
        let files: BTreeMap<String, String> = files
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect();
        let seen = Arc::clone(&lookups);
        let open: BundleOpener = Arc::new(move || {
            Ok(Box::new(FakeBundle {
                digest,
                files: files.clone(),
                lookups: Arc::clone(&seen),
            }) as Box<dyn Bundle>)
        });
        (PackedBundle::new(pack, open), lookups)
    }

    fn read(bundle: &mut PackedBundle, name: &str) -> Option<String> {
        match bundle.input_open_name(name, &mut NoopStatusBackend::default()) {
            OpenResult::Ok(mut handle) => {
                let mut text = String::new();
                handle.read_to_string(&mut text).unwrap();
                Some(text)
            }
            OpenResult::NotAvailable => None,
            OpenResult::Err(error) => panic!("lookup of {name} failed: {error}"),
        }
    }

    #[test]
    fn pack_files_never_reach_the_bundle() {
        let dir = tempfile::tempdir().unwrap();
        write_pack(dir.path(), &[("article.cls", "from the pack")]);
        let (mut bundle, lookups) = packed(
            dir.path(),
            PACK_DIGEST,
            &[
                ("article.cls", "from the bundle"),
                ("siunitx.sty", "from the bundle"),
            ],
        );

        assert_eq!(
            read(&mut bundle, "article.cls").as_deref(),
            Some("from the pack")
        );
        assert_eq!(
            read(&mut bundle, "siunitx.sty").as_deref(),
            Some("from the bundle")
        );
        assert_eq!(read(&mut bundle, "absent.sty"), None);

        let asked: Vec<String> = lookups
            .lock()
            .unwrap()
            .iter()
            .filter(|name| *name != INDEX_PROBE)
            .cloned()
            .collect();
        assert_eq!(asked, ["siunitx.sty", "absent.sty"]);
    }

    #[test]
    fn a_different_bundle_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_pack(dir.path(), &[]);
        let (mut bundle, _) = packed(dir.path(), OTHER_DIGEST, &[("siunitx.sty", "")]);

        match bundle.input_open_name("siunitx.sty", &mut NoopStatusBackend::default()) {
            OpenResult::Err(error) => assert_eq!(
                error.to_string(),
                format!(
                    "the support pack was built for bundle {PACK_DIGEST}, but the bundle is now {OTHER_DIGEST}"
                )
            ),
            _ => panic!("a mismatched bundle was used"),
        }
    }

    #[test]
    fn manifest_names_stay_inside_the_pack() {
        let dir = tempfile::tempdir().unwrap();
        write_pack(dir.path(), &[]);
        let text = fs::read_to_string(dir.path().join("manifest.json")).unwrap();
        let escaped = text.replace("\"files\":{}", "\"files\":{\"../secret\":1}");
        fs::write(dir.path().join("manifest.json"), escaped).unwrap();

        match SupportPack::open(dir.path()) {
            Err(PackError::Name(name)) => assert_eq!(name, "../secret"),
            other => panic!("expected a name error, got {other:?}"),
        }
    }

    #[test]
    fn the_format_is_seeded_only_for_its_engine() {
        let dir = tempfile::tempdir().unwrap();
        write_pack(dir.path(), &[("latex.fmt", "dump")]);
        let pack = SupportPack::open(dir.path()).unwrap();
        let formats = tempfile::tempdir().unwrap();
        let mut cache =
            FormatCache::new(pack.bundle_digest(), formats.path().to_owned(), SERIAL + 1);
        let mut status = NoopStatusBackend::default();

        pack.seed_format(&mut cache, "latex.fmt", SERIAL + 1)
            .unwrap();
        assert!(matches!(
            cache.input_open_format("latex.fmt", &mut status),
            OpenResult::NotAvailable
        ));

        let mut cache = FormatCache::new(pack.bundle_digest(), formats.path().to_owned(), SERIAL);
        pack.seed_format(&mut cache, "latex.fmt", SERIAL).unwrap();
        let OpenResult::Ok(mut handle) = cache.input_open_format("latex.fmt", &mut status) else {
            panic!("the format was not seeded");
        };
        let mut text = String::new();
        handle.read_to_string(&mut text).unwrap();
        assert_eq!(text, "dump");
    }
}
