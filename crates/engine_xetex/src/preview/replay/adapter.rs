//! Byte channels over the immutable in-memory TeX sources.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::mem;
use std::path::Path;
use std::sync::Arc;

use super::source::{ResolvedSource, SourceIdentity, SourcePath, SourceSnapshot};
use crate::ffi::bridge::FileFormat;

thread_local! {
    static ACTIVE: RefCell<Option<ReplaySources>> = const { RefCell::new(None) };
    static OBSERVED_BYTES: Cell<usize> = const { Cell::new(0) };
}

struct ReplaySources {
    snapshot: SourceSnapshot,
    inputs: Vec<MemoryInput>,
    last_path: Option<SourcePath>,
    written: BTreeSet<SourcePath>,
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

pub(crate) struct SourceInstallGuard<'a> {
    written: &'a mut BTreeSet<SourcePath>,
}

impl Drop for SourceInstallGuard<'_> {
    fn drop(&mut self) {
        if let Some(sources) = ACTIVE.with(|active| active.borrow_mut().take()) {
            *self.written = sources.written;
        }
        OBSERVED_BYTES.set(0);
    }
}

/// Serve `snapshot` to the engine. `written` lists the files the engine has
/// written so far and grows with any it writes until the guard drops.
pub(crate) fn install(
    snapshot: SourceSnapshot,
    written: &mut BTreeSet<SourcePath>,
) -> SourceInstallGuard<'_> {
    OBSERVED_BYTES.set(0);
    ACTIVE.with(|active| {
        let previous = active.borrow_mut().replace(ReplaySources {
            snapshot,
            inputs: Vec::new(),
            last_path: None,
            written: mem::take(written),
        });
        assert!(previous.is_none(), "nested source snapshot installation");
    });
    SourceInstallGuard { written }
}

/// Note a file the engine opened for writing, so later reads get its output and not a disk copy.
pub(crate) fn record_written(path: &str) {
    ACTIVE.with(|active| {
        if let Some(sources) = active.borrow_mut().as_mut()
            && let Some(path) = sources.snapshot.source_path(Path::new(path))
        {
            sources.written.insert(path);
        }
    });
}

pub(crate) fn checkpoint_inputs() -> Option<Vec<MemoryInput>> {
    ACTIVE.with(|active| Some(active.borrow().as_ref()?.inputs.clone()))
}

pub(crate) fn restore_inputs(checkpoints: &[MemoryInput]) -> bool {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(sources) = active.as_mut() else {
            return false;
        };
        if !sources.inputs.is_empty() {
            return false;
        }
        let mut restored = Vec::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            let Some((bytes, identity)) = sources.snapshot.replay_entry(&checkpoint.path) else {
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
        sources.inputs = restored;
        OBSERVED_BYTES.set(
            checkpoints
                .iter()
                .fold(0usize, |total, input| total.saturating_add(input.seen)),
        );
        true
    })
}

/// Cumulative source bytes observed by the installed byte channels.
///
/// Unlike the token clock, this stays still while TeX expands already-read
/// tokens.
pub(crate) fn observed_bytes() -> usize {
    OBSERVED_BYTES.get()
}

pub(crate) fn open_primary() -> Option<MemoryInputId> {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active.as_mut()?;
        let primary = sources.snapshot.primary_entry();
        Some(sources.open(primary))
    })
}

pub(crate) fn open_named(path: &str, format: FileFormat) -> Option<MemoryInputId> {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active.as_mut()?;
        let requested = Path::new(path);
        let resolved = match sources.snapshot.resolve_entry(requested) {
            Some(resolved) => resolved,
            None => {
                if format != FileFormat::Tex {
                    return None;
                }
                let written = sources
                    .snapshot
                    .source_path(requested)
                    .is_some_and(|path| sources.written.contains(&path));
                if written {
                    return None;
                }
                sources.snapshot.resolve_replay_entry(requested)?
            }
        };
        Some(sources.open(resolved))
    })
}

pub(crate) fn holds_directory(dir: &str) -> bool {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .is_some_and(|sources| sources.snapshot.holds_directory(dir))
    })
}

pub(crate) fn last_abspath(max_len: usize) -> Option<Vec<u8>> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let sources = active.as_ref()?;
        let path = sources.last_path.as_ref()?;
        let path = sources.snapshot.absolute_path(path);
        let bytes = path.to_string_lossy();
        Some(bytes.as_bytes()[..bytes.len().min(max_len)].to_vec())
    })
}

/// A snapshot input opened through the installed sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoryInputId(usize);

/// The whole file behind `id` and the position its reader stands at.
pub(crate) fn contents(id: MemoryInputId) -> (Arc<[u8]>, usize) {
    with_memory_input(id, |input| (input.bytes.clone(), input.position))
}

/// Moves `id`'s reader to `position` and records that the engine has looked at
/// every byte below `reached`.
pub(crate) fn sync(id: MemoryInputId, position: usize, reached: usize) {
    with_memory_input(id, |input| {
        input.position = position;
        observe_through(input, reached);
    });
}

/// Closes `id`, returning whether it was already closed.
pub(crate) fn close(id: MemoryInputId) -> bool {
    with_input(id, |input| mem::replace(&mut input.closed, true))
}

pub(crate) fn checkpoint_input_is_open(checkpoints: &[MemoryInput], id: MemoryInputId) -> bool {
    checkpoints
        .get(id.0)
        .is_some_and(|checkpoint| !checkpoint.closed)
}

pub(crate) fn open_position(path: &SourcePath) -> Option<usize> {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()?
            .inputs
            .iter()
            .rev()
            .find(|input| !input.closed && input.path == *path)
            .map(|input| input.position)
    })
}

/// Answer a size query without recording a dependency, since the cursor does
/// not move. A document that typesets such a size keeps a stale value until
/// the next full compile.
pub(crate) fn get_size(id: MemoryInputId) -> usize {
    with_memory_input(id, |input| input.bytes.len())
}

/// Fills `data` from `id`, returning whether it was filled.
pub(crate) fn read(id: MemoryInputId, data: &mut [u8]) -> bool {
    let len = data.len();
    with_memory_input(id, |input| {
        let available = input.bytes.len().saturating_sub(input.position);
        let copied = len.min(available);
        data[..copied].copy_from_slice(&input.bytes[input.position..input.position + copied]);
        input.position += copied;
        observe_through(input, input.position);
        copied == len
    })
}

fn observe_through(input: &mut MemoryInput, position: usize) {
    let newly_observed = position.saturating_sub(input.seen);
    input.seen = input.seen.max(position);
    OBSERVED_BYTES.set(OBSERVED_BYTES.get().saturating_add(newly_observed));
}

/// Moves `id` to byte `target`, returning the new position, or 0 past the end.
pub(crate) fn seek(id: MemoryInputId, target: usize) -> usize {
    with_memory_input(id, |input| {
        if target > input.bytes.len() {
            // The engine learned the file's extent, so hold the fence to all of it.
            observe_through(input, input.bytes.len());
            return 0;
        }
        input.position = target;
        observe_through(input, input.position);
        input.position
    })
}

impl ReplaySources {
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
}

fn with_input<T>(id: MemoryInputId, f: impl FnOnce(&mut MemoryInput) -> T) -> T {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active
            .as_mut()
            .expect("memory input without installed snapshot");
        f(&mut sources.inputs[id.0])
    })
}

fn with_memory_input<T>(id: MemoryInputId, f: impl FnOnce(&mut MemoryInput) -> T) -> T {
    with_input(id, |input| {
        assert!(!input.closed, "use of closed memory input");
        f(input)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn snapshot() -> SourceSnapshot {
        SourceSnapshot::new(
            PathBuf::from("/workspace"),
            PathBuf::from("main.tex"),
            vec![
                (PathBuf::from("main.tex"), Arc::from(&b"abc"[..])),
                (PathBuf::from("child.tex"), Arc::from(&b"xyz"[..])),
            ],
        )
        .unwrap()
    }

    fn byte(id: MemoryInputId) -> Option<u8> {
        let mut buf = [0u8; 1];
        read(id, &mut buf).then_some(buf[0])
    }

    #[test]
    fn inputs_match_seek_and_exact_read_semantics() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let id = open_primary().unwrap();
        assert_eq!(get_size(id), 3);
        assert_eq!(byte(id), Some(b'a'));
        assert_eq!(seek(id, 1), 1);
        let mut buf = [0u8; 2];
        assert!(read(id, &mut buf));
        assert_eq!(&buf, b"bc");
        assert_eq!(byte(id), None);
        assert!(!close(id));
        assert!(close(id));
    }

    #[test]
    fn primary_and_child_are_memory_backed_from_the_first_open() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let primary = open_primary().unwrap();
        let child = open_named("child", FileFormat::Tex).unwrap();
        assert_ne!(primary, child);
        assert_eq!(byte(primary), Some(b'a'));
        assert_eq!(byte(child), Some(b'x'));
    }

    #[test]
    fn checkpoint_recreates_handle_order_and_cursor() {
        let mut written = BTreeSet::new();
        let (checkpoints, primary, child) = {
            let _guard = install(snapshot(), &mut written);
            let primary = open_primary().unwrap();
            let child = open_named("child", FileFormat::Tex).unwrap();
            assert_eq!(byte(primary), Some(b'a'));
            assert_eq!(byte(child), Some(b'x'));
            assert_eq!(byte(child), Some(b'y'));
            (checkpoint_inputs().unwrap(), primary, child)
        };

        let _guard = install(snapshot(), &mut written);
        assert!(restore_inputs(&checkpoints));
        assert_eq!(byte(primary), Some(b'b'));
        assert_eq!(byte(child), Some(b'z'));
    }

    #[test]
    fn observed_byte_clock_advances_only_for_new_source_bytes() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let primary = open_primary().unwrap();
        assert_eq!(observed_bytes(), 0);
        assert_eq!(byte(primary), Some(b'a'));
        assert_eq!(observed_bytes(), 1);
        assert_eq!(seek(primary, 0), 0);
        assert_eq!(byte(primary), Some(b'a'));
        assert_eq!(observed_bytes(), 1);
        let child = open_named("child", FileFormat::Tex).unwrap();
        let mut buf = [0u8; 2];
        assert!(read(child, &mut buf));
        assert_eq!(observed_bytes(), 3);

        let checkpoints = checkpoint_inputs().unwrap();
        drop(_guard);
        let _restored_guard = install(snapshot(), &mut written);
        assert!(restore_inputs(&checkpoints));
        assert_eq!(observed_bytes(), 3);
    }
}
