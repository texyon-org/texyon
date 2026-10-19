//! Support-file I/O and status reporting backed by the JavaScript host.

use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, SeekFrom},
    rc::Rc,
};

use texyon_io_base::{InputFeatures, InputHandle, InputOrigin, IoProvider, OpenResult};
use texyon_status_base::{MessageKind, StatusBackend};

use crate::host::{self, LogKind};

/// Bytes of one support file, shared by every handle opened on it.
pub type FileBytes = Rc<[u8]>;

/// A read cursor over shared bytes.
///
/// `Cursor<Vec<u8>>` would copy the file on every open, and every replay
/// reopens the same fonts and classes.
struct SharedFile {
    bytes: FileBytes,
    pos: usize,
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (&self.bytes[self.pos..]).read(buf)?;
        self.pos += n;
        Ok(n)
    }
}

impl InputFeatures for SharedFile {
    fn get_size(&mut self) -> texyon_errors::Result<usize> {
        Ok(self.bytes.len())
    }

    fn try_seek(&mut self, pos: SeekFrom) -> texyon_errors::Result<u64> {
        let len = self.bytes.len();
        let target = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => (len as u64).checked_add_signed(offset),
            SeekFrom::Current(offset) => (self.pos as u64).checked_add_signed(offset),
        };

        let Some(target) = target else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot seek before the start of the file",
            )
            .into());
        };

        // Seeking past the end is legal and reads nothing, as with a real file.
        self.pos = usize::try_from(target).map_or(len, |target| target.min(len));
        Ok(self.pos as u64)
    }
}

/// An [`IoProvider`] that resolves support files through the host.
///
/// Every lookup, including misses, is cached for the session because a miss
/// is a blocking network round trip. Clones share the cache, so the engine
/// and the renderer never fetch the same file twice.
#[derive(Default, Clone)]
pub struct HostIo {
    cache: Rc<RefCell<HashMap<String, Option<FileBytes>>>>,
}

impl HostIo {
    /// Seed the cache with a file the host already has.
    pub fn preload(&self, name: impl Into<String>, bytes: impl Into<FileBytes>) {
        self.cache
            .borrow_mut()
            .insert(name.into(), Some(bytes.into()));
    }

    /// Distinct support files resident.
    pub fn cached_count(&self) -> usize {
        self.cache.borrow().values().flatten().count()
    }

    /// Total bytes of resident support files.
    pub fn cached_bytes(&self) -> usize {
        self.cache
            .borrow()
            .values()
            .flatten()
            .map(|b| b.len())
            .sum()
    }

    /// Read a support file, going to the host on a miss.
    pub fn read(&self, name: &str) -> Result<Option<FileBytes>, String> {
        if let Some(hit) = self.cache.borrow().get(name) {
            return Ok(hit.clone());
        }

        let bytes = host::read_file(name)?.map(FileBytes::from);
        self.cache
            .borrow_mut()
            .insert(name.to_owned(), bytes.clone());

        Ok(bytes)
    }
}

impl IoProvider for HostIo {
    fn input_open_name(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        match self.read(name) {
            Ok(Some(bytes)) => OpenResult::Ok(InputHandle::new_read_only(
                name,
                SharedFile { bytes, pos: 0 },
                InputOrigin::Other,
            )),
            Ok(None) => OpenResult::NotAvailable,
            Err(error) => OpenResult::Err(texyon_errors::anyhow::anyhow!(error)),
        }
    }
}

/// Forwards engine diagnostics to the host and appends them to a shared
/// transcript, since a status backend outlives nothing it could return to.
pub struct HostStatus {
    lines: Rc<RefCell<Vec<String>>>,
}

impl HostStatus {
    pub fn new(lines: Rc<RefCell<Vec<String>>>) -> Self {
        Self { lines }
    }
}

impl StatusBackend for HostStatus {
    fn report(
        &mut self,
        kind: MessageKind,
        args: std::fmt::Arguments,
        err: Option<&texyon_errors::Error>,
    ) {
        let mut message = args.to_string();

        if let Some(err) = err {
            for cause in err.chain() {
                message.push_str("\ncaused by: ");
                message.push_str(&cause.to_string());
            }
        }

        host::log(
            match kind {
                MessageKind::Note => LogKind::Note,
                MessageKind::Warning => LogKind::Warning,
                MessageKind::Error => LogKind::Error,
            },
            &message,
        );

        self.lines.borrow_mut().push(message);
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        let text = String::from_utf8_lossy(output).into_owned();
        host::log(LogKind::Error, &text);
        self.lines.borrow_mut().push(text);
    }
}
