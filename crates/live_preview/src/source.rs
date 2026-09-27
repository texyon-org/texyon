use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt, fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use tectonic_preview_protocol::{ProtocolLimits, SourceFile, SourceRevision};

use tectonic_engine_xetex::{
    SourceError as EngineSourceError, SourceRevision as EngineSourceRevision,
    SourceSnapshot as EngineSourceSnapshot,
};

/// What, if anything, stands behind a document root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Backing {
    /// A real directory.
    ///
    /// Paths are resolved *through* it, so a child that reaches outside the
    /// root by way of a symlink is refused even though its lexical path stays
    /// inside.
    Filesystem,
    /// Nothing.
    ///
    /// Every source is held in memory and the root is only a name that paths
    /// are confined under. This is what the browser uses, where there is no
    /// filesystem to canonicalize against — and where there are correspondingly
    /// no symlinks, so lexical normalization is the whole of the authority.
    Detached,
}

/// The authority within which preview sources may exist.
#[derive(Clone, Debug)]
pub struct DocumentRoot {
    canonical: PathBuf,
    backing: Backing,
}

impl DocumentRoot {
    /// Canonicalize an existing document-root directory.
    ///
    /// Sources under this root may be read from disk, and paths resolve
    /// through the real filesystem.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        let canonical = fs::canonicalize(path.as_ref()).map_err(SourceError::Io)?;
        if !canonical.is_dir() {
            return Err(SourceError::RootIsNotDirectory(canonical));
        }
        Ok(Self {
            canonical,
            backing: Backing::Filesystem,
        })
    }

    /// Take an absolute path as a root with no filesystem behind it.
    ///
    /// Nothing is canonicalized and nothing is required to exist: the path is
    /// a name that every source is confined under, and every source has to be
    /// supplied in full. Use this where there is no filesystem to consult —
    /// which is the browser, and any host that owns the document itself.
    ///
    /// The path must already be absolute, since there is no working directory
    /// to make it absolute against.
    pub fn detached(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(SourceError::InvalidPath(
                path.to_string_lossy().into_owned(),
            ));
        }
        Ok(Self {
            canonical: normalize_absolute(path)?,
            backing: Backing::Detached,
        })
    }

    /// Canonical root path used for authority checks.
    pub fn path(&self) -> &Path {
        &self.canonical
    }

    fn resolve(&self, logical: &str) -> Result<PathBuf, SourceError> {
        if logical.is_empty() || logical.as_bytes().contains(&0) {
            return Err(SourceError::InvalidPath(logical.into()));
        }
        let mut path = PathBuf::from(logical);
        if path.extension().is_none() {
            path.set_extension("tex");
        }
        let candidate = if path.is_absolute() {
            path
        } else {
            self.canonical.join(&path)
        };
        let normalized = normalize_absolute(&candidate)?;
        let resolved = match self.backing {
            Backing::Filesystem => resolve_existing_ancestor(&normalized)?,
            // No filesystem means no symlink can redirect a path out of the
            // root, so the lexical form already is the resolved form.
            Backing::Detached => normalized,
        };
        if !resolved.starts_with(&self.canonical) {
            return Err(SourceError::OutsideRoot {
                path: logical.into(),
                resolved,
            });
        }
        Ok(resolved)
    }
}

/// Immutable accepted source snapshot passed to one engine operation.
#[derive(Clone, Debug)]
pub struct AcceptedSourceSnapshot {
    revision: SourceRevision,
    root: PathBuf,
    primary: PathBuf,
    files: Arc<BTreeMap<PathBuf, Arc<str>>>,
    engine_sources: EngineSourceSnapshot,
}

impl AcceptedSourceSnapshot {
    /// Accepted source revision carried by this immutable snapshot.
    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// Canonical document root governing this snapshot's authority.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical primary path.
    pub fn primary(&self) -> &Path {
        &self.primary
    }

    /// Canonical source paths and complete UTF-8 buffers.
    pub fn files(&self) -> &BTreeMap<PathBuf, Arc<str>> {
        &self.files
    }

    /// Convert to the engine's immutable source adapter without changing revision identity.
    pub fn engine_snapshot(&self) -> EngineSourceSnapshot {
        self.engine_sources.clone()
    }
}

/// Mutable accepted source state owned by the preview worker.
#[derive(Clone, Debug)]
pub struct SourceSet {
    root: DocumentRoot,
    primary: PathBuf,
    files: Arc<BTreeMap<PathBuf, Arc<str>>>,
    total_bytes: usize,
    engine_sources: EngineSourceSnapshot,
    revision: SourceRevision,
    limits: ProtocolLimits,
}

impl SourceSet {
    /// Validate and stage a complete initialization snapshot at revision zero.
    pub fn initialize(
        root: DocumentRoot,
        primary_path: &str,
        primary_text: String,
        children: Vec<SourceFile>,
        limits: ProtocolLimits,
    ) -> Result<Self, SourceError> {
        validate_file_size(&primary_text, limits)?;
        let primary = root.resolve(primary_path)?;
        let mut files = BTreeMap::<PathBuf, Arc<str>>::new();
        files.insert(primary.clone(), Arc::from(primary_text));
        let mut total = files[&primary].len();

        for child in children {
            validate_file_size(&child.text, limits)?;
            let path = root.resolve(&child.path)?;
            if files.insert(path.clone(), Arc::from(child.text)).is_some() {
                return Err(SourceError::DuplicatePath(path));
            }
            total = total.saturating_add(files[&path].len());
        }
        validate_snapshot_size(total, limits)?;
        let engine_sources = EngineSourceSnapshot::new_with_revision(
            root.canonical.clone(),
            primary.clone(),
            files
                .iter()
                .map(|(path, text)| (path.clone(), Arc::<[u8]>::from(text.as_bytes())))
                .collect(),
            EngineSourceRevision::new(SourceRevision::ZERO.get()),
        )
        .map_err(SourceError::Engine)?;

        Ok(Self {
            root,
            primary,
            files: Arc::new(files),
            total_bytes: total,
            engine_sources,
            revision: SourceRevision::ZERO,
            limits,
        })
    }

    /// Current accepted source revision.
    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// Canonical primary path.
    pub fn primary(&self) -> &Path {
        &self.primary
    }

    /// Replace a complete primary or child buffer and advance exactly once.
    ///
    /// Byte-identical accepted replacements still advance the source revision.
    pub fn replace(
        &mut self,
        path: Option<&str>,
        text: String,
    ) -> Result<SourceRevision, SourceError> {
        validate_file_size(&text, self.limits)?;
        let path = match path {
            Some(path) => self.root.resolve(path)?,
            None => self.primary.clone(),
        };
        let previous_len = self.files.get(&path).map_or(0, |text| text.len());
        let total = self
            .total_bytes
            .saturating_sub(previous_len)
            .saturating_add(text.len());
        validate_snapshot_size(total, self.limits)?;
        let revision = self
            .revision
            .checked_next()
            .map_err(|_| SourceError::RevisionExhausted)?;
        let text: Arc<str> = Arc::from(text);
        let engine_update = self
            .engine_sources
            .upsert_with_revision(
                &path,
                Arc::<[u8]>::from(text.as_bytes()),
                EngineSourceRevision::new(revision.get()),
            )
            .map_err(SourceError::Engine)?;

        Arc::make_mut(&mut self.files).insert(path, text);
        self.total_bytes = total;
        self.engine_sources = engine_update.snapshot;
        self.revision = revision;
        Ok(revision)
    }

    /// Freeze the current accepted source state for an engine operation.
    pub fn snapshot(&self) -> AcceptedSourceSnapshot {
        AcceptedSourceSnapshot {
            revision: self.revision,
            root: self.root.canonical.clone(),
            primary: self.primary.clone(),
            files: Arc::clone(&self.files),
            engine_sources: self.engine_sources.for_operation(),
        }
    }
}

/// Source authority, sizing, or revision failure.
#[derive(Debug)]
pub enum SourceError {
    /// Filesystem resolution failed.
    Io(std::io::Error),
    /// Configured root exists but is not a directory.
    RootIsNotDirectory(PathBuf),
    /// Logical path is empty, contains NUL, or cannot normalize absolutely.
    InvalidPath(String),
    /// Resolved path escapes the configured canonical root.
    OutsideRoot {
        /// Requested logical path.
        path: String,
        /// Resolved path demonstrating the escaped authority.
        resolved: PathBuf,
    },
    /// Initialization names the same resolved source more than once.
    DuplicatePath(PathBuf),
    /// One complete source buffer exceeds its configured decoded limit.
    FileTooLarge {
        /// Source UTF-8 byte count.
        bytes: usize,
        /// Configured source-file limit.
        limit: usize,
    },
    /// Combined accepted source buffers exceed their configured limit.
    SnapshotTooLarge {
        /// Combined source UTF-8 byte count.
        bytes: usize,
        /// Configured snapshot limit.
        limit: usize,
    },
    /// Accepted source revision space is exhausted.
    RevisionExhausted,
    /// Engine source normalization rejected a coordinator-validated path.
    Engine(EngineSourceError),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "source resolution failed: {error}"),
            Self::RootIsNotDirectory(path) => {
                write!(f, "document root {} is not a directory", path.display())
            }
            Self::InvalidPath(path) => write!(f, "invalid source path {path:?}"),
            Self::OutsideRoot { path, resolved } => write!(
                f,
                "source path {path:?} resolves outside the document root to {}",
                resolved.display()
            ),
            Self::DuplicatePath(path) => {
                write!(f, "source path {} appears more than once", path.display())
            }
            Self::FileTooLarge { bytes, limit } => {
                write!(f, "source file has {bytes} bytes, exceeding {limit}")
            }
            Self::SnapshotTooLarge { bytes, limit } => {
                write!(f, "source snapshot has {bytes} bytes, exceeding {limit}")
            }
            Self::RevisionExhausted => write!(f, "source revision counter exhausted"),
            Self::Engine(error) => write!(f, "engine source snapshot failed: {error}"),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Engine(error) => Some(error),
            _ => None,
        }
    }
}

fn validate_file_size(text: &str, limits: ProtocolLimits) -> Result<(), SourceError> {
    if text.len() > limits.max_source_file_bytes {
        return Err(SourceError::FileTooLarge {
            bytes: text.len(),
            limit: limits.max_source_file_bytes,
        });
    }
    Ok(())
}

fn validate_snapshot_size(total: usize, limits: ProtocolLimits) -> Result<(), SourceError> {
    if total > limits.max_source_snapshot_bytes {
        return Err(SourceError::SnapshotTooLarge {
            bytes: total,
            limit: limits.max_source_snapshot_bytes,
        });
    }
    Ok(())
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, SourceError> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(SourceError::InvalidPath(
                        path.to_string_lossy().into_owned(),
                    ));
                }
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    if !normalized.is_absolute() {
        return Err(SourceError::InvalidPath(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(normalized)
}

fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf, SourceError> {
    let mut ancestor = path.to_path_buf();
    let mut missing = Vec::<OsString>::new();
    let canonical = loop {
        match fs::canonicalize(&ancestor) {
            Ok(canonical) => break canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor
                    .file_name()
                    .ok_or_else(|| SourceError::InvalidPath(path.to_string_lossy().into_owned()))?;
                missing.push(name.to_os_string());
                if !ancestor.pop() {
                    return Err(SourceError::InvalidPath(
                        path.to_string_lossy().into_owned(),
                    ));
                }
            }
            Err(error) => return Err(SourceError::Io(error)),
        }
    };
    let mut resolved = canonical;
    for name in missing.iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProtocolLimits {
        ProtocolLimits {
            max_source_file_bytes: 32,
            max_source_snapshot_bytes: 48,
            max_json_line_bytes: 1024,
            max_frame_bytes: 1024,
        }
    }

    #[test]
    fn a_detached_root_needs_no_filesystem() {
        // The path deliberately does not exist. A detached root is a name, so
        // nothing here should ever reach the disk to find that out.
        let root = DocumentRoot::detached("/doc").unwrap();
        let sources = SourceSet::initialize(
            root,
            "main.tex",
            "text".into(),
            vec![SourceFile {
                path: "chapters/one.tex".into(),
                text: "child".into(),
            }],
            limits(),
        )
        .unwrap();

        let snapshot = sources.snapshot();
        assert_eq!(snapshot.primary(), Path::new("/doc/main.tex"));
        assert!(snapshot
            .files()
            .contains_key(Path::new("/doc/chapters/one.tex")));
    }

    #[test]
    fn a_detached_root_still_confines_its_children() {
        let root = DocumentRoot::detached("/doc").unwrap();
        let mut sources =
            SourceSet::initialize(root, "main.tex", "text".into(), vec![], limits()).unwrap();

        // Confinement is the whole point of the root, and it does not depend on
        // there being a filesystem to check against.
        assert!(sources
            .replace(Some("../escape.tex"), "bad".into())
            .is_err());
        assert!(sources
            .replace(Some("/elsewhere/escape.tex"), "bad".into())
            .is_err());
        assert_eq!(sources.revision(), SourceRevision::ZERO);
    }

    #[test]
    fn a_detached_root_must_be_absolute() {
        // There is no working directory to resolve a relative root against.
        assert!(DocumentRoot::detached("doc").is_err());
    }

    #[test]
    fn accepted_identical_replacement_advances_revision() {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let mut sources =
            SourceSet::initialize(root, "main.tex", "same".into(), vec![], limits()).unwrap();

        assert_eq!(
            sources.replace(None, "same".into()).unwrap(),
            SourceRevision::new(1)
        );
        let snapshot = sources.snapshot();
        assert_eq!(snapshot.revision(), SourceRevision::new(1));
        assert_eq!(snapshot.engine_snapshot().revision().get(), 1);
    }

    #[test]
    fn rejected_replacement_does_not_mutate_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let mut sources =
            SourceSet::initialize(root, "main.tex", "good".into(), vec![], limits()).unwrap();
        let before = sources.snapshot();

        assert!(sources
            .replace(Some("../escape.tex"), "bad".into())
            .is_err());
        let after = sources.snapshot();
        assert_eq!(after.revision(), SourceRevision::ZERO);
        assert_eq!(after.files(), before.files());
    }

    #[cfg(unix)]
    #[test]
    fn existing_symlink_escape_is_rejected() {
        use std::os::unix::fs::symlink;

        let root_dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root_dir.path().join("linked")).unwrap();
        let root = DocumentRoot::new(root_dir.path()).unwrap();

        assert!(matches!(
            root.resolve("linked/dirty.tex"),
            Err(SourceError::OutsideRoot { .. })
        ));
    }

    #[test]
    fn duplicate_normalized_children_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("sub")).unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let children = vec![
            SourceFile {
                path: "sub/child.tex".into(),
                text: "a".into(),
            },
            SourceFile {
                path: "sub/../sub/child.tex".into(),
                text: "b".into(),
            },
        ];

        assert!(matches!(
            SourceSet::initialize(root, "main.tex", "main".into(), children, limits()),
            Err(SourceError::DuplicatePath(_))
        ));
    }

    #[test]
    fn implicit_tex_name_replaces_existing_child_authority() {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let mut sources = SourceSet::initialize(
            root,
            "main",
            "main".into(),
            vec![SourceFile {
                path: "child.tex".into(),
                text: "old".into(),
            }],
            limits(),
        )
        .unwrap();

        sources.replace(Some("child"), "new".into()).unwrap();
        let snapshot = sources.snapshot();
        assert_eq!(snapshot.files().len(), 2);
        assert_eq!(
            snapshot
                .files()
                .get(&temp.path().join("child.tex"))
                .unwrap()
                .as_ref(),
            "new"
        );
    }

    #[test]
    fn first_valid_replacement_registers_editor_owned_child() {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let mut sources =
            SourceSet::initialize(root, "main.tex", "main".into(), vec![], limits()).unwrap();

        assert_eq!(
            sources
                .replace(Some("later/child"), "child".into())
                .unwrap(),
            SourceRevision::new(1)
        );
        let snapshot = sources.snapshot();
        assert_eq!(snapshot.files().len(), 2);
        assert_eq!(
            snapshot
                .files()
                .get(&temp.path().join("later/child.tex"))
                .unwrap()
                .as_ref(),
            "child"
        );
    }

    #[test]
    fn large_snapshot_edits_reuse_unmodified_engine_buffers() {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let children = (0..512)
            .map(|index| SourceFile {
                path: format!("chapters/{index}.tex"),
                text: format!("chapter {index}"),
            })
            .collect();
        let mut sources = SourceSet::initialize(
            root,
            "main.tex",
            "main".into(),
            children,
            ProtocolLimits::default(),
        )
        .unwrap();
        let before = sources.snapshot().engine_snapshot();
        let untouched_before = before.bytes("chapters/511.tex").unwrap();

        sources
            .replace(Some("chapters/0.tex"), "edited chapter".into())
            .unwrap();
        let accepted = sources.snapshot();
        let after = accepted.engine_snapshot();
        assert!(Arc::ptr_eq(
            &untouched_before,
            &after.bytes("chapters/511.tex").unwrap()
        ));
        assert!(Arc::ptr_eq(
            &after.bytes("chapters/511.tex").unwrap(),
            &accepted
                .engine_snapshot()
                .bytes("chapters/511.tex")
                .unwrap()
        ));
    }
}
