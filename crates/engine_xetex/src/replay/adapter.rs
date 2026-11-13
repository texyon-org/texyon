//! Tagged byte handles for immutable in-memory TeX sources.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use super::source::{ResolvedSource, SourceIdentity, SourcePath, SourceSnapshot};
use crate::bridge::FileFormat;
use texyon_bridge_core::EOF;

const MEM_TAG: usize = 1usize << (usize::BITS - 1);
const INDEX_MASK: usize = !MEM_TAG;

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
    pub(crate) pushback: Option<u8>,
    /// Furthest position ever reached by reading or seeking. Bytes below it
    /// are consumed facts.
    pub(crate) seen: usize,
    pub(crate) closed: bool,
}

/// Uninstalls the sources and hands the written paths back to their owner.
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
            written: std::mem::take(written),
        });
        assert!(previous.is_none(), "nested source snapshot installation");
    });
    SourceInstallGuard { written }
}

/// Note a file the engine opened for writing, so later reads get its output and not a disk copy.
pub(crate) fn record_written(path: &str) {
    ACTIVE.with(|active| {
        if let Some(sources) = active.borrow_mut().as_mut() {
            if let Some(path) = sources.snapshot.source_path(Path::new(path)) {
                sources.written.insert(path);
            }
        }
    });
}

/// Every opened or previously consumed editor byte channel, in handle order.
pub(crate) fn checkpoint_inputs() -> Option<Vec<MemoryInput>> {
    ACTIVE.with(|active| Some(active.borrow().as_ref()?.inputs.clone()))
}

/// Recreate a captured handle table against the currently installed snapshot.
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
                pushback: checkpoint.pushback,
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

pub(crate) fn open_primary() -> Option<usize> {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active.as_mut()?;
        let primary = sources.snapshot.primary_entry();
        Some(sources.open(primary))
    })
}

pub(crate) fn open_named(path: &str, format: FileFormat) -> Option<usize> {
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

/// Whether the active snapshot holds a buffer in directory `dir`.
pub(crate) fn holds_directory(dir: &str) -> bool {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .is_some_and(|sources| sources.snapshot.holds_directory(dir))
    })
}

pub(crate) fn clear_last_path() {
    ACTIVE.with(|active| {
        if let Some(active) = active.borrow_mut().as_mut() {
            active.last_path = None;
        }
    });
}

/// Whether replay opened the last input, which it does only under the exact requested name.
pub(crate) fn opened_last() -> bool {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .is_some_and(|sources| sources.last_path.is_some())
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

pub(crate) fn is_mem(handle: usize) -> bool {
    handle & MEM_TAG != 0
}

pub(crate) fn checkpoint_handle_is_open(checkpoints: &[MemoryInput], handle: usize) -> bool {
    is_mem(handle)
        && checkpoints
            .get(index(handle))
            .is_some_and(|checkpoint| !checkpoint.closed)
}

/// Underlying byte cursor of the newest open channel for `path`.
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
pub(crate) fn get_size(handle: usize) -> Option<usize> {
    with_open_input(handle, |input| input.bytes.len())
}

pub(crate) fn getc(handle: usize) -> Option<libc::c_int> {
    with_open_input(handle, |input| {
        if let Some(byte) = input.pushback.take() {
            return byte.into();
        }
        let Some(&byte) = input.bytes.get(input.position) else {
            // The engine acted on the file ending here, so appending
            // invalidates the fence.
            observe_through(input, input.position);
            return EOF;
        };
        input.position += 1;
        observe_through(input, input.position);
        byte.into()
    })
}

pub(crate) fn ungetc(handle: usize, ch: libc::c_int) -> Option<libc::c_int> {
    with_open_input(handle, |input| {
        if input.pushback.replace(ch as u8).is_some() {
            -1
        } else {
            0
        }
    })
}

pub(crate) fn read(handle: usize, data: &mut [u8]) -> Option<libc::ssize_t> {
    let len = data.len();
    with_open_input(handle, |input| {
        let mut copied = 0;
        if len > 0 {
            if let Some(byte) = input.pushback.take() {
                data[0] = byte;
                copied = 1;
            }
        }
        let available = input.bytes.len().saturating_sub(input.position);
        let direct = (len - copied).min(available);
        data[copied..copied + direct]
            .copy_from_slice(&input.bytes[input.position..input.position + direct]);
        input.position += direct;
        observe_through(input, input.position);
        copied += direct;
        if copied == len {
            len as libc::ssize_t
        } else {
            -1
        }
    })
}

fn observe_through(input: &mut MemoryInput, position: usize) {
    let newly_observed = position.saturating_sub(input.seen);
    input.seen = input.seen.max(position);
    OBSERVED_BYTES.set(OBSERVED_BYTES.get().saturating_add(newly_observed));
}

pub(crate) fn seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int) -> Option<usize> {
    with_open_input(handle, |input| {
        let base = match whence {
            libc::SEEK_SET => 0i128,
            libc::SEEK_CUR => input.position as i128,
            libc::SEEK_END => input.bytes.len() as i128,
            _ => return 0,
        };
        let target = base + offset as i128;
        if !(0..=input.bytes.len() as i128).contains(&target) {
            // The engine learned the file's extent, so hold the fence to all of it.
            observe_through(input, input.bytes.len());
            return 0;
        }
        input.position = target as usize;
        observe_through(input, input.position);
        input.pushback = None;
        input.position
    })
}

/// Returns whether the channel was already closed.
pub(crate) fn close(handle: usize) -> Option<bool> {
    with_input(handle, |input| std::mem::replace(&mut input.closed, true))
}

impl ReplaySources {
    fn open(&mut self, (path, bytes, identity): ResolvedSource) -> usize {
        self.inputs.push(MemoryInput {
            path: path.clone(),
            bytes,
            identity,
            position: 0,
            pushback: None,
            seen: 0,
            closed: false,
        });
        self.last_path = Some(path);
        MEM_TAG | self.inputs.len()
    }
}

fn with_input<T>(handle: usize, f: impl FnOnce(&mut MemoryInput) -> T) -> Option<T> {
    if !is_mem(handle) {
        return None;
    }
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active
            .as_mut()
            .expect("memory handle without installed snapshot");
        Some(f(&mut sources.inputs[index(handle)]))
    })
}

fn with_open_input<T>(handle: usize, f: impl FnOnce(&mut MemoryInput) -> T) -> Option<T> {
    with_input(handle, |input| {
        assert!(!input.closed, "use of closed memory input");
        f(input)
    })
}

fn index(handle: usize) -> usize {
    let raw = handle & INDEX_MASK;
    assert!(raw > 0, "zero memory input index");
    raw - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn tagged_inputs_match_getc_ungetc_seek_and_exact_read_semantics() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let handle = open_primary().unwrap();
        assert!(is_mem(handle));
        assert_eq!(get_size(handle), Some(3));
        assert_eq!(getc(handle), Some(b'a'.into()));
        assert_eq!(ungetc(handle, b'Q'.into()), Some(0));
        assert_eq!(getc(handle), Some(b'Q'.into()));
        assert_eq!(seek(handle, 1, libc::SEEK_SET), Some(1));
        let mut buf = [0u8; 2];
        assert_eq!(read(handle, &mut buf), Some(2));
        assert_eq!(&buf, b"bc");
        assert_eq!(getc(handle), Some(EOF));
        assert_eq!(close(handle), Some(false));
    }

    #[test]
    fn primary_and_child_are_memory_backed_from_the_first_open() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let primary = open_primary().unwrap();
        let child = open_named("child", FileFormat::Tex).unwrap();
        assert!(is_mem(primary));
        assert!(is_mem(child));
        assert_eq!(getc(primary), Some(b'a'.into()));
        assert_eq!(getc(child), Some(b'x'.into()));
    }

    #[test]
    fn checkpoint_recreates_handle_order_cursor_and_pushback() {
        let mut written = BTreeSet::new();
        let checkpoints = {
            let _guard = install(snapshot(), &mut written);
            let primary = open_primary().unwrap();
            let child = open_named("child", FileFormat::Tex).unwrap();
            assert_eq!(getc(primary), Some(b'a'.into()));
            assert_eq!(getc(child), Some(b'x'.into()));
            assert_eq!(ungetc(child, b'Q'.into()), Some(0));
            checkpoint_inputs().unwrap()
        };

        let _guard = install(snapshot(), &mut written);
        assert!(restore_inputs(&checkpoints));
        assert_eq!(getc(MEM_TAG | 1), Some(b'b'.into()));
        assert_eq!(getc(MEM_TAG | 2), Some(b'Q'.into()));
        assert_eq!(getc(MEM_TAG | 2), Some(b'y'.into()));
    }

    #[test]
    fn observed_byte_clock_advances_only_for_new_source_bytes() {
        let mut written = BTreeSet::new();
        let _guard = install(snapshot(), &mut written);
        let primary = open_primary().unwrap();
        assert_eq!(observed_bytes(), 0);
        assert_eq!(getc(primary), Some(b'a'.into()));
        assert_eq!(observed_bytes(), 1);
        assert_eq!(seek(primary, 0, libc::SEEK_SET), Some(0));
        assert_eq!(getc(primary), Some(b'a'.into()));
        assert_eq!(observed_bytes(), 1);
        let child = open_named("child", FileFormat::Tex).unwrap();
        let mut buf = [0u8; 2];
        assert_eq!(read(child, &mut buf), Some(2));
        assert_eq!(observed_bytes(), 3);

        let checkpoints = checkpoint_inputs().unwrap();
        drop(_guard);
        let _restored_guard = install(snapshot(), &mut written);
        assert!(restore_inputs(&checkpoints));
        assert_eq!(observed_bytes(), 3);
    }
}
