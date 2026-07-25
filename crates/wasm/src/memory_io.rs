//! An in-memory sink for everything the engines write.
//!
//! A browser session has no output directory: the `.xdv` is handed straight to
//! xdvipdfmx, the `.pdf` is handed straight to JavaScript, and the `.log` is
//! shown in the UI. Nothing needs to reach a filesystem, so the whole output
//! side of the I/O stack lives here.

use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc};

use tectonic_io_base::{InputHandle, InputOrigin, IoProvider, OpenResult, OutputHandle};
use tectonic_status_base::StatusBackend;

/// The name under which the engine's terminal output is filed.
///
/// Not a name TeX can produce itself, so it cannot collide with a real output.
pub const STDOUT_NAME: &str = "<stdout>";

/// Files written during a session, shared with whoever wants to read them back.
type Files = Rc<RefCell<HashMap<String, Vec<u8>>>>;

/// The buffer an [`OutputHandle`] writes into, which commits to the map on drop.
struct FileSink {
    name: String,
    files: Files,
    data: Vec<u8>,
}

impl std::io::Write for FileSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        // Publish as we go, so a file the engine leaves open — the `.log`
        // during a failed run — is still readable afterwards.
        self.files
            .borrow_mut()
            .insert(self.name.clone(), self.data.clone());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An [`IoProvider`] that keeps every written file in memory.
#[derive(Default, Clone)]
pub struct MemoryIo {
    files: Files,
}

impl MemoryIo {
    /// Construct an empty in-memory filesystem.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take a written file's contents, leaving the map without it.
    pub fn take(&self, name: &str) -> Option<Vec<u8>> {
        self.files.borrow_mut().remove(name)
    }

    /// Read a written file's contents without removing it.
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.files.borrow().get(name).cloned()
    }

    /// Install a file as though the engine had written it.
    pub fn insert(&self, name: impl Into<String>, data: Vec<u8>) {
        self.files.borrow_mut().insert(name.into(), data);
    }

    /// Forget every file.
    pub fn clear(&self) {
        self.files.borrow_mut().clear();
    }
}

impl IoProvider for MemoryIo {
    fn output_open_name(&mut self, name: &str) -> OpenResult<OutputHandle> {
        OpenResult::Ok(OutputHandle::new(
            name,
            FileSink {
                name: name.to_owned(),
                files: Rc::clone(&self.files),
                data: Vec::new(),
            },
        ))
    }

    fn output_open_stdout(&mut self) -> OpenResult<OutputHandle> {
        // TeX opens the terminal early and writes its banner and progress to
        // it. Refusing the open hands the engine a null handle, which it does
        // not check — so this has to be a real sink even though nothing reads
        // it back except a caller asking for STDOUT_NAME.
        self.output_open_name(STDOUT_NAME)
    }

    fn input_open_name(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        match self.files.borrow().get(name) {
            Some(data) => OpenResult::Ok(InputHandle::new_read_only(
                name,
                Cursor::new(data.clone()),
                InputOrigin::Other,
            )),
            None => OpenResult::NotAvailable,
        }
    }
}
