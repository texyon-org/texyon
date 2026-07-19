//! Immutable, revisioned editor source snapshots.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use sha2::{Digest, Sha256};

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// A globally monotonic accepted source-content revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRevision(u64);

impl SourceRevision {
    /// Return the numeric revision for logging and protocol serialization.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// A normalized, root-relative TeX source path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(Arc<str>);

impl SourcePath {
    /// Return the normalized slash-separated path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Strong identity of the exact source bytes consumed by XeTeX.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceIdentity([u8; 32]);

impl SourceIdentity {
    /// Return the SHA-256 digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A validated immutable set of editor-owned source buffers.
#[derive(Clone)]
pub struct SourceSnapshot {
    root: Arc<PathBuf>,
    revision: SourceRevision,
    primary: SourcePath,
    entries: Arc<BTreeMap<SourcePath, SourceEntry>>,
}

#[derive(Clone)]
struct SourceEntry {
    bytes: Arc<[u8]>,
    identity: SourceIdentity,
}

/// Result of replacing one whole editor buffer.
pub struct SourceUpdate {
    /// The immutable snapshot for the next engine operation.
    pub snapshot: SourceSnapshot,
    /// The accepted normalized path, absent for a byte-identical no-op.
    pub changed_path: Option<SourcePath>,
    /// First differing byte against the previous accepted buffer.
    pub first_changed: Option<usize>,
}

/// Validation error for an editor-owned source path or replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceError {
    /// The configured source root must be absolute.
    RootNotAbsolute,
    /// A source escaped the configured root or used a parent component.
    OutsideRoot(PathBuf),
    /// A source normalized to an empty path.
    EmptyPath,
    /// Two supplied paths normalized to the same TeX path.
    DuplicatePath(SourcePath),
    /// The primary source was not included in the supplied entries.
    PrimaryMissing(SourcePath),
    /// A replacement targeted a path not owned by the editor snapshot.
    UnknownPath(SourcePath),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotAbsolute => f.write_str("source root must be absolute"),
            Self::OutsideRoot(path) => write!(f, "source path escapes root: {}", path.display()),
            Self::EmptyPath => f.write_str("source path is empty"),
            Self::DuplicatePath(path) => write!(f, "duplicate source path: {path}"),
            Self::PrimaryMissing(path) => write!(f, "primary source is missing: {path}"),
            Self::UnknownPath(path) => write!(f, "source path is not editor-owned: {path}"),
        }
    }
}

impl std::error::Error for SourceError {}

impl SourceSnapshot {
    /// Validate and freeze a complete source set. Paths may be relative to
    /// `root` or absolute beneath it; extensionless TeX paths normalize to
    /// their implicit `.tex` spelling.
    pub fn new(
        root: PathBuf,
        primary: PathBuf,
        sources: Vec<(PathBuf, Arc<[u8]>)>,
    ) -> Result<Self, SourceError> {
        if !root.is_absolute() {
            return Err(SourceError::RootNotAbsolute);
        }
        let primary = normalize(&root, &primary)?;
        let mut entries = BTreeMap::new();
        for (path, bytes) in sources {
            let path = normalize(&root, &path)?;
            let identity = identity(&bytes);
            if entries
                .insert(path.clone(), SourceEntry { bytes, identity })
                .is_some()
            {
                return Err(SourceError::DuplicatePath(path));
            }
        }
        if !entries.contains_key(&primary) {
            return Err(SourceError::PrimaryMissing(primary));
        }
        Ok(Self {
            root: Arc::new(root),
            revision: next_revision(),
            primary,
            entries: Arc::new(entries),
        })
    }

    /// The immutable snapshot revision.
    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// The normalized primary source path.
    pub fn primary(&self) -> &SourcePath {
        &self.primary
    }

    /// Resolve editor-owned bytes, including XeTeX's implicit `.tex` spelling.
    pub fn bytes(&self, path: impl AsRef<Path>) -> Option<Arc<[u8]>> {
        let path = normalize(&self.root, path.as_ref()).ok()?;
        self.entries.get(&path).map(|entry| Arc::clone(&entry.bytes))
    }

    /// Resolve the strong identity of editor-owned source bytes.
    pub fn identity(&self, path: impl AsRef<Path>) -> Option<SourceIdentity> {
        let path = normalize(&self.root, path.as_ref()).ok()?;
        self.entries.get(&path).map(|entry| entry.identity)
    }

    /// Replace one complete known buffer and return a new immutable snapshot.
    /// A byte-identical replacement is a no-op and retains the revision.
    pub fn replace(
        &self,
        path: impl AsRef<Path>,
        bytes: Arc<[u8]>,
    ) -> Result<SourceUpdate, SourceError> {
        let path = normalize(&self.root, path.as_ref())?;
        let old = self
            .entries
            .get(&path)
            .ok_or_else(|| SourceError::UnknownPath(path.clone()))?;
        let first_changed = first_difference(&old.bytes, &bytes);
        if first_changed.is_none() {
            return Ok(SourceUpdate {
                snapshot: self.clone(),
                changed_path: None,
                first_changed: None,
            });
        }
        let mut entries = (*self.entries).clone();
        entries.insert(
            path.clone(),
            SourceEntry {
                identity: identity(&bytes),
                bytes,
            },
        );
        Ok(SourceUpdate {
            snapshot: Self {
                root: Arc::clone(&self.root),
                revision: next_revision(),
                primary: self.primary.clone(),
                entries: Arc::new(entries),
            },
            changed_path: Some(path),
            first_changed,
        })
    }
}

fn next_revision() -> SourceRevision {
    SourceRevision(NEXT_REVISION.fetch_add(1, Ordering::Relaxed))
}

fn identity(bytes: &[u8]) -> SourceIdentity {
    SourceIdentity(Sha256::digest(bytes).into())
}

fn first_difference(left: &[u8], right: &[u8]) -> Option<usize> {
    let shared = left.len().min(right.len());
    left[..shared]
        .iter()
        .zip(&right[..shared])
        .position(|(left, right)| left != right)
        .or_else(|| (left.len() != right.len()).then_some(shared))
}

fn normalize(root: &Path, path: &Path) -> Result<SourcePath, SourceError> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root)
            .map_err(|_| SourceError::OutsideRoot(path.to_owned()))?
    } else {
        path
    };
    let mut clean = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => clean.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SourceError::OutsideRoot(path.to_owned()));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(SourceError::EmptyPath);
    }
    if clean.extension().is_none() {
        clean.set_extension("tex");
    }
    let normalized = clean
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(SourcePath(Arc::from(normalized)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: &[u8]) -> Arc<[u8]> {
        Arc::from(value)
    }

    fn snapshot() -> SourceSnapshot {
        SourceSnapshot::new(
            PathBuf::from("/workspace"),
            PathBuf::from("main"),
            vec![
                (PathBuf::from("main.tex"), bytes(b"abc")),
                (PathBuf::from("chapters/one.tex"), bytes(b"child")),
            ],
        )
        .unwrap()
    }

    #[test]
    fn normalizes_implicit_tex_paths_and_freezes_bytes() {
        let snapshot = snapshot();
        assert_eq!(snapshot.primary().as_str(), "main.tex");
        assert_eq!(&*snapshot.bytes("main").unwrap(), b"abc");
        assert_eq!(
            snapshot.identity("main"),
            snapshot.identity("/workspace/main.tex")
        );
    }

    #[test]
    fn replacement_records_only_path_and_first_difference() {
        let snapshot = snapshot();
        let update = snapshot.replace("main", bytes(b"abXYZ")).unwrap();
        assert_eq!(update.changed_path.unwrap().as_str(), "main.tex");
        assert_eq!(update.first_changed, Some(2));
        assert!(update.snapshot.revision() > snapshot.revision());
        assert_eq!(&*snapshot.bytes("main").unwrap(), b"abc");
        assert_eq!(&*update.snapshot.bytes("main").unwrap(), b"abXYZ");
    }

    #[test]
    fn rejects_outside_and_unknown_paths_without_mutating_snapshot() {
        let snapshot = snapshot();
        let revision = snapshot.revision();
        assert!(matches!(
            snapshot.replace("../escape.tex", bytes(b"x")),
            Err(SourceError::OutsideRoot(_))
        ));
        assert!(matches!(
            snapshot.replace("other.tex", bytes(b"x")),
            Err(SourceError::UnknownPath(_))
        ));
        assert_eq!(snapshot.revision(), revision);
        assert_eq!(&*snapshot.bytes("main").unwrap(), b"abc");
    }

    #[test]
    fn byte_identical_replacement_keeps_revision() {
        let snapshot = snapshot();
        let update = snapshot.replace("main.tex", bytes(b"abc")).unwrap();
        assert_eq!(update.snapshot.revision(), snapshot.revision());
        assert!(update.changed_path.is_none());
        assert!(update.first_changed.is_none());
    }
}
