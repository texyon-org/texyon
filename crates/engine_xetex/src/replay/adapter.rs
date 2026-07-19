//! Tagged byte handles for immutable in-memory TeX sources.

use std::cell::RefCell;
use std::ffi::CStr;
use std::path::Path;
use std::sync::Arc;

use super::source::{SourceIdentity, SourcePath, SourceSnapshot};

const MEM_TAG: usize = 1usize << (usize::BITS - 1);
const INDEX_MASK: usize = !MEM_TAG;

thread_local! {
    static ACTIVE: RefCell<Option<ReplaySources>> = const { RefCell::new(None) };
}

struct ReplaySources {
    snapshot: SourceSnapshot,
    inputs: Vec<MemInput>,
    last_path: Option<SourcePath>,
}

struct MemInput {
    path: SourcePath,
    bytes: Arc<[u8]>,
    position: usize,
    pushback: Option<u8>,
    closed: bool,
}

/// Replayable byte-channel state captured atomically at a command fence.
#[derive(Clone)]
pub(crate) struct MemoryInputCheckpoint {
    pub(crate) path: SourcePath,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) identity: SourceIdentity,
    pub(crate) position: usize,
    pub(crate) pushback: Option<u8>,
    pub(crate) closed: bool,
}

pub(crate) struct SourceInstallGuard;

impl Drop for SourceInstallGuard {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            active.borrow_mut().take();
        });
    }
}

pub(crate) fn install(snapshot: SourceSnapshot) -> SourceInstallGuard {
    ACTIVE.with(|active| {
        let previous = active.borrow_mut().replace(ReplaySources {
            snapshot,
            inputs: Vec::new(),
            last_path: None,
        });
        assert!(previous.is_none(), "nested source snapshot installation");
    });
    SourceInstallGuard
}

/// Capture every opened or previously consumed editor byte channel in handle order.
pub(crate) fn checkpoint_inputs() -> Option<Vec<MemoryInputCheckpoint>> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let sources = active.as_ref()?;
        Some(
            sources
                .inputs
                .iter()
                .map(|input| MemoryInputCheckpoint {
                    path: input.path.clone(),
                    bytes: Arc::clone(&input.bytes),
                    identity: sources
                        .snapshot
                        .entry(&input.path)
                        .expect("open memory input disappeared from snapshot")
                        .1,
                    position: input.position,
                    pushback: input.pushback,
                    closed: input.closed,
                })
                .collect(),
        )
    })
}

/// Recreate a captured handle table against the currently installed snapshot.
pub(crate) fn restore_inputs(checkpoints: &[MemoryInputCheckpoint]) -> bool {
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
            let Some((bytes, _)) = sources.snapshot.entry(&checkpoint.path) else {
                return false;
            };
            if checkpoint.position > bytes.len() {
                return false;
            }
            restored.push(MemInput {
                path: checkpoint.path.clone(),
                bytes,
                position: checkpoint.position,
                pushback: checkpoint.pushback,
                closed: checkpoint.closed,
            });
        }
        sources.inputs = restored;
        true
    })
}

pub(crate) fn open_primary() -> Option<usize> {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active.as_mut()?;
        let (path, bytes) = sources.snapshot.primary_entry();
        Some(sources.open(path, bytes))
    })
}

pub(crate) unsafe fn open_named(path: *const libc::c_char) -> Option<usize> {
    if path.is_null() {
        return None;
    }
    let path = CStr::from_ptr(path).to_string_lossy();
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active.as_mut()?;
        let (path, bytes) = sources.snapshot.resolve_entry(Path::new(path.as_ref()))?;
        Some(sources.open(path, bytes))
    })
}

pub(crate) fn clear_last_path() {
    ACTIVE.with(|active| {
        if let Some(active) = active.borrow_mut().as_mut() {
            active.last_path = None;
        }
    });
}

pub(crate) unsafe fn copy_last_abspath(
    buffer: *mut libc::c_char,
    len: usize,
) -> Option<libc::ssize_t> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let sources = active.as_ref()?;
        let path = sources.last_path.as_ref()?;
        let path = sources.snapshot.absolute_path(path);
        let bytes = path.to_string_lossy();
        if len == 0 {
            return Some(0);
        }
        let copied = bytes.len().min(len - 1);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast::<u8>(), copied);
        *buffer.add(copied) = 0;
        Some(copied as libc::ssize_t)
    })
}

pub(crate) fn is_mem(handle: usize) -> bool {
    handle & MEM_TAG != 0
}

pub(crate) fn checkpoint_handle_is_open(
    checkpoints: &[MemoryInputCheckpoint],
    handle: usize,
) -> bool {
    if !is_mem(handle) {
        return false;
    }
    checkpoints
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

pub(crate) fn get_size(handle: usize) -> Option<usize> {
    with_input(handle, |input| input.bytes.len())
}

pub(crate) fn getc(handle: usize) -> Option<libc::c_int> {
    with_input_mut(handle, |input| {
        if let Some(byte) = input.pushback.take() {
            return byte.into();
        }
        let Some(&byte) = input.bytes.get(input.position) else {
            return libc::EOF;
        };
        input.position += 1;
        byte.into()
    })
}

pub(crate) fn ungetc(handle: usize, ch: libc::c_int) -> Option<libc::c_int> {
    with_input_mut(handle, |input| {
        if input.pushback.replace(ch as u8).is_some() {
            -1
        } else {
            0
        }
    })
}

pub(crate) unsafe fn read(handle: usize, data: *mut u8, len: usize) -> Option<libc::ssize_t> {
    with_input_mut(handle, |input| {
        let mut copied = 0;
        if len > 0 {
            if let Some(byte) = input.pushback.take() {
                *data = byte;
                copied = 1;
            }
        }
        let available = input.bytes.len().saturating_sub(input.position);
        let wanted = len - copied;
        let direct = wanted.min(available);
        std::ptr::copy_nonoverlapping(
            input.bytes.as_ptr().add(input.position),
            data.add(copied),
            direct,
        );
        input.position += direct;
        copied += direct;
        if copied == len {
            len as libc::ssize_t
        } else {
            -1
        }
    })
}

pub(crate) fn seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int) -> Option<usize> {
    with_input_mut(handle, |input| {
        let base = match whence {
            libc::SEEK_SET => 0i128,
            libc::SEEK_CUR => input.position as i128,
            libc::SEEK_END => input.bytes.len() as i128,
            _ => return 0,
        };
        let target = base + offset as i128;
        if !(0..=input.bytes.len() as i128).contains(&target) {
            return 0;
        }
        input.position = target as usize;
        input.pushback = None;
        input.position
    })
}

pub(crate) fn close(handle: usize) -> Option<bool> {
    if !is_mem(handle) {
        return None;
    }
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active
            .as_mut()
            .expect("memory handle without installed snapshot");
        let input = &mut sources.inputs[index(handle)];
        let was_closed = input.closed;
        input.closed = true;
        Some(was_closed)
    })
}

impl ReplaySources {
    fn open(&mut self, path: SourcePath, bytes: Arc<[u8]>) -> usize {
        self.inputs.push(MemInput {
            path: path.clone(),
            bytes,
            position: 0,
            pushback: None,
            closed: false,
        });
        self.last_path = Some(path);
        MEM_TAG | self.inputs.len()
    }
}

fn with_input<T>(handle: usize, f: impl FnOnce(&MemInput) -> T) -> Option<T> {
    if !is_mem(handle) {
        return None;
    }
    ACTIVE.with(|active| {
        let active = active.borrow();
        let sources = active
            .as_ref()
            .expect("memory handle without installed snapshot");
        let input = &sources.inputs[index(handle)];
        assert!(!input.closed, "use of closed memory input");
        Some(f(input))
    })
}

fn with_input_mut<T>(handle: usize, f: impl FnOnce(&mut MemInput) -> T) -> Option<T> {
    if !is_mem(handle) {
        return None;
    }
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let sources = active
            .as_mut()
            .expect("memory handle without installed snapshot");
        let input = &mut sources.inputs[index(handle)];
        assert!(!input.closed, "use of closed memory input");
        Some(f(input))
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
        let _guard = install(snapshot());
        let handle = open_primary().unwrap();
        assert!(is_mem(handle));
        assert_eq!(get_size(handle), Some(3));
        assert_eq!(getc(handle), Some(b'a'.into()));
        assert_eq!(ungetc(handle, b'Q'.into()), Some(0));
        assert_eq!(getc(handle), Some(b'Q'.into()));
        assert_eq!(seek(handle, 1, libc::SEEK_SET), Some(1));
        let mut buf = [0u8; 2];
        assert_eq!(unsafe { read(handle, buf.as_mut_ptr(), 2) }, Some(2));
        assert_eq!(&buf, b"bc");
        assert_eq!(getc(handle), Some(libc::EOF));
        assert_eq!(close(handle), Some(false));
    }

    #[test]
    fn primary_and_child_are_memory_backed_from_the_first_open() {
        let _guard = install(snapshot());
        let primary = open_primary().unwrap();
        let child = unsafe { open_named(c"child".as_ptr()) }.unwrap();
        assert!(is_mem(primary));
        assert!(is_mem(child));
        assert_eq!(getc(primary), Some(b'a'.into()));
        assert_eq!(getc(child), Some(b'x'.into()));
    }

    #[test]
    fn checkpoint_recreates_handle_order_cursor_and_pushback() {
        let checkpoints = {
            let _guard = install(snapshot());
            let primary = open_primary().unwrap();
            let child = unsafe { open_named(c"child".as_ptr()) }.unwrap();
            assert_eq!(getc(primary), Some(b'a'.into()));
            assert_eq!(getc(child), Some(b'x'.into()));
            assert_eq!(ungetc(child, b'Q'.into()), Some(0));
            checkpoint_inputs().unwrap()
        };

        let _guard = install(snapshot());
        assert!(restore_inputs(&checkpoints));
        assert_eq!(getc(MEM_TAG | 1), Some(b'b'.into()));
        assert_eq!(getc(MEM_TAG | 2), Some(b'Q'.into()));
        assert_eq!(getc(MEM_TAG | 2), Some(b'y'.into()));
    }
}
