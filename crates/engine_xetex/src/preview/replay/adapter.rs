//! Byte channels over the immutable in-memory TeX sources of one operation.

use std::collections::BTreeSet;
use std::mem;
use std::path::Path;
use std::sync::Arc;

use super::source::{ResolvedSource, SourceIdentity, SourcePath, SourceSnapshot};
use crate::ffi::bridge::FileFormat;

/// The snapshot an operation reads its sources from, with the files it opened.
pub(crate) struct SnapshotSources {
    snapshot: SourceSnapshot,
    inputs: Vec<MemoryInput>,
    last_path: Option<SourcePath>,
    /// Files the engine has written, read back from its output and not from here.
    written: BTreeSet<SourcePath>,
    /// Source bytes looked at so far. Unlike the token clock, this stays still
    /// while TeX expands tokens it already read.
    observed: usize,
}

/// One byte channel. Fences capture the whole table by cloning it.
#[derive(Clone)]
pub(crate) struct MemoryInput {
    pub(crate) path: SourcePath,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) identity: SourceIdentity,
    pub(crate) position: usize,
    /// Furthest position ever reached by reading or seeking. Bytes below it
    /// are consumed facts.
    pub(crate) seen: usize,
    pub(crate) closed: bool,
}

/// A snapshot input opened through [`SnapshotSources`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoryInputId(usize);

impl MemoryInput {
    fn observe_through(&mut self, position: usize) -> usize {
        let newly_observed = position.saturating_sub(self.seen);
        self.seen = self.seen.max(position);
        newly_observed
    }
}

impl SnapshotSources {
    /// Serves `snapshot`. `written` lists the files the engine has written so far.
    pub(crate) fn new(snapshot: SourceSnapshot, written: BTreeSet<SourcePath>) -> Self {
        SnapshotSources {
            snapshot,
            inputs: Vec::new(),
            last_path: None,
            written,
            observed: 0,
        }
    }

    /// The files the engine has written, including any from this operation.
    pub(crate) fn into_written(self) -> BTreeSet<SourcePath> {
        self.written
    }

    /// Notes a file the engine opened for writing, so later reads get its output and not a disk copy.
    pub(crate) fn record_written(&mut self, path: &str) {
        if let Some(path) = self.snapshot.source_path(Path::new(path)) {
            self.written.insert(path);
        }
    }

    pub(crate) fn checkpoint(&self) -> Vec<MemoryInput> {
        self.inputs.clone()
    }

    /// Reopens the files of `checkpoints` over this snapshot, false if one is gone.
    pub(crate) fn restore(&mut self, checkpoints: &[MemoryInput]) -> bool {
        if !self.inputs.is_empty() {
            return false;
        }
        let mut restored = Vec::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            let Some((bytes, identity)) = self.snapshot.replay_entry(&checkpoint.path) else {
                return false;
            };
            if checkpoint.position > bytes.len() {
                return false;
            }
            restored.push(MemoryInput {
                path: checkpoint.path.clone(),
                bytes,
                identity,
                position: checkpoint.position,
                seen: checkpoint.seen,
                closed: checkpoint.closed,
            });
        }
        self.inputs = restored;
        self.observed = checkpoints
            .iter()
            .fold(0usize, |total, input| total.saturating_add(input.seen));
        true
    }

    /// Cumulative source bytes observed by the open channels.
    pub(crate) fn observed_bytes(&self) -> usize {
        self.observed
    }

    pub(crate) fn open_primary(&mut self) -> MemoryInputId {
        let primary = self.snapshot.primary_entry();
        self.open(primary)
    }

    /// Opens `path` from the snapshot. A `.tex` file under the root that the
    /// editor does not hold is read from disk once, unless the engine wrote it.
    pub(crate) fn open_named(&mut self, path: &str, format: FileFormat) -> Option<MemoryInputId> {
        let requested = Path::new(path);
        let resolved = match self.snapshot.resolve_entry(requested) {
            Some(resolved) => resolved,
            None => {
                if format != FileFormat::Tex {
                    return None;
                }
                let written = self
                    .snapshot
                    .source_path(requested)
                    .is_some_and(|path| self.written.contains(&path));
                if written {
                    return None;
                }
                self.snapshot.resolve_replay_entry(requested)?
            }
        };
        Some(self.open(resolved))
    }

    pub(crate) fn holds_directory(&self, dir: &str) -> bool {
        self.snapshot.holds_directory(dir)
    }

    /// The absolute path of the last file opened, at most `max_len` bytes.
    pub(crate) fn last_abspath(&self, max_len: usize) -> Option<Vec<u8>> {
        let path = self.snapshot.absolute_path(self.last_path.as_ref()?);
        let bytes = path.to_string_lossy();
        Some(bytes.as_bytes()[..bytes.len().min(max_len)].to_vec())
    }

    /// Where the last open channel on `path` stands.
    pub(crate) fn open_position(&self, path: &SourcePath) -> Option<usize> {
        self.inputs
            .iter()
            .rev()
            .find(|input| !input.closed && input.path == *path)
            .map(|input| input.position)
    }

    /// The whole file behind `id` and the position its reader stands at.
    pub(crate) fn contents(&mut self, id: MemoryInputId) -> Option<(Arc<[u8]>, usize)> {
        let input = self.open_input(id)?;
        Some((input.bytes.clone(), input.position))
    }

    /// Moves `id`'s reader to `position` and records that the engine has looked
    /// at every byte below `reached`.
    pub(crate) fn sync(&mut self, id: MemoryInputId, position: usize, reached: usize) {
        if let Some(input) = self.open_input(id) {
            input.position = position;
            let newly = input.observe_through(reached);
            self.observed = self.observed.saturating_add(newly);
        }
    }

    /// Closes `id`, returning whether it was already closed.
    pub(crate) fn close(&mut self, id: MemoryInputId) -> bool {
        self.inputs
            .get_mut(id.0)
            .is_none_or(|input| mem::replace(&mut input.closed, true))
    }

    /// The size of `id`, answered without recording a dependency since the
    /// cursor does not move. A document that typesets such a size keeps a stale
    /// value until the next full compile.
    pub(crate) fn size(&mut self, id: MemoryInputId) -> usize {
        self.open_input(id).map_or(0, |input| input.bytes.len())
    }

    /// Fills `data` from `id`, returning whether it was filled.
    pub(crate) fn read(&mut self, id: MemoryInputId, data: &mut [u8]) -> bool {
        let len = data.len();
        let Some(input) = self.open_input(id) else {
            return false;
        };
        let available = input.bytes.len().saturating_sub(input.position);
        let copied = len.min(available);
        data[..copied].copy_from_slice(&input.bytes[input.position..input.position + copied]);
        input.position += copied;
        let newly = input.observe_through(input.position);
        self.observed = self.observed.saturating_add(newly);
        copied == len
    }

    /// Moves `id` to byte `target`, returning the new position, or 0 past the end.
    pub(crate) fn seek(&mut self, id: MemoryInputId, target: usize) -> usize {
        let Some(input) = self.open_input(id) else {
            return 0;
        };
        let (newly, position) = if target > input.bytes.len() {
            // The engine learned the file's extent, so hold the fence to all of it.
            (input.observe_through(input.bytes.len()), 0)
        } else {
            input.position = target;
            (input.observe_through(target), target)
        };
        self.observed = self.observed.saturating_add(newly);
        position
    }

    fn open(&mut self, (path, bytes, identity): ResolvedSource) -> MemoryInputId {
        self.inputs.push(MemoryInput {
            path: path.clone(),
            bytes,
            identity,
            position: 0,
            seen: 0,
            closed: false,
        });
        self.last_path = Some(path);
        MemoryInputId(self.inputs.len() - 1)
    }

    fn open_input(&mut self, id: MemoryInputId) -> Option<&mut MemoryInput> {
        self.inputs.get_mut(id.0).filter(|input| !input.closed)
    }
}

pub(crate) fn checkpoint_input_is_open(checkpoints: &[MemoryInput], id: MemoryInputId) -> bool {
    checkpoints
        .get(id.0)
        .is_some_and(|checkpoint| !checkpoint.closed)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn sources() -> SnapshotSources {
        let snapshot = SourceSnapshot::new(
            PathBuf::from("/workspace"),
            PathBuf::from("main.tex"),
            vec![
                (PathBuf::from("main.tex"), Arc::from(&b"abc"[..])),
                (PathBuf::from("child.tex"), Arc::from(&b"xyz"[..])),
            ],
        )
        .unwrap();
        SnapshotSources::new(snapshot, BTreeSet::new())
    }

    fn byte(sources: &mut SnapshotSources, id: MemoryInputId) -> Option<u8> {
        let mut buf = [0u8; 1];
        sources.read(id, &mut buf).then_some(buf[0])
    }

    #[test]
    fn inputs_match_seek_and_exact_read_semantics() {
        let mut sources = sources();
        let id = sources.open_primary();
        assert_eq!(sources.size(id), 3);
        assert_eq!(byte(&mut sources, id), Some(b'a'));
        assert_eq!(sources.seek(id, 1), 1);
        let mut buf = [0u8; 2];
        assert!(sources.read(id, &mut buf));
        assert_eq!(&buf, b"bc");
        assert_eq!(byte(&mut sources, id), None);
        assert!(!sources.close(id));
        assert!(sources.close(id));
    }

    #[test]
    fn primary_and_child_are_memory_backed_from_the_first_open() {
        let mut sources = sources();
        let primary = sources.open_primary();
        let child = sources.open_named("child", FileFormat::Tex).unwrap();
        assert_ne!(primary, child);
        assert_eq!(byte(&mut sources, primary), Some(b'a'));
        assert_eq!(byte(&mut sources, child), Some(b'x'));
    }

    #[test]
    fn checkpoint_recreates_handle_order_and_cursor() {
        let mut first = sources();
        let primary = first.open_primary();
        let child = first.open_named("child", FileFormat::Tex).unwrap();
        assert_eq!(byte(&mut first, primary), Some(b'a'));
        assert_eq!(byte(&mut first, child), Some(b'x'));
        assert_eq!(byte(&mut first, child), Some(b'y'));
        let checkpoints = first.checkpoint();

        let mut restored = sources();
        assert!(restored.restore(&checkpoints));
        assert_eq!(byte(&mut restored, primary), Some(b'b'));
        assert_eq!(byte(&mut restored, child), Some(b'z'));
    }

    #[test]
    fn observed_byte_clock_advances_only_for_new_source_bytes() {
        let mut first = sources();
        let primary = first.open_primary();
        assert_eq!(first.observed_bytes(), 0);
        assert_eq!(byte(&mut first, primary), Some(b'a'));
        assert_eq!(first.observed_bytes(), 1);
        assert_eq!(first.seek(primary, 0), 0);
        assert_eq!(byte(&mut first, primary), Some(b'a'));
        assert_eq!(first.observed_bytes(), 1);
        let child = first.open_named("child", FileFormat::Tex).unwrap();
        let mut buf = [0u8; 2];
        assert!(first.read(child, &mut buf));
        assert_eq!(first.observed_bytes(), 3);

        let mut restored = sources();
        assert!(restored.restore(&first.checkpoint()));
        assert_eq!(restored.observed_bytes(), 3);
    }

    #[test]
    fn a_written_file_is_not_read_from_the_disk_copy() {
        let root = std::env::temp_dir().join(format!("texyon-sources-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("notes.tex"), b"from disk").unwrap();
        let snapshot = SourceSnapshot::new(
            root.clone(),
            PathBuf::from("main.tex"),
            vec![(PathBuf::from("main.tex"), Arc::from(&b"abc"[..]))],
        )
        .unwrap();
        let mut sources = SnapshotSources::new(snapshot, BTreeSet::new());
        assert!(sources.open_named("notes", FileFormat::Tex).is_some());
        sources.record_written(&root.join("notes.tex").to_string_lossy());
        assert_eq!(sources.open_named("notes", FileFormat::Tex), None);
        let written = sources.into_written();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(written.iter().any(|path| path.as_str() == "notes.tex"));
    }
}
