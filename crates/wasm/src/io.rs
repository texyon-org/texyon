//! Support-file I/O backed by the JavaScript host, and a status backend that
//! forwards engine diagnostics to it.

use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, SeekFrom},
    rc::Rc,
};

use tectonic_io_base::{InputFeatures, InputHandle, InputOrigin, IoProvider, OpenResult};
use tectonic_status_base::{MessageKind, StatusBackend};

use crate::host::{self, LogKind};

/// Bytes of one support file, shared by every handle opened on it.
type FileBytes = Rc<[u8]>;

/// A read cursor over shared support-file bytes.
///
/// The obvious spelling, `Cursor<Vec<u8>>`, is the one `tectonic_io_base`
/// implements [`InputFeatures`] for, but it would copy the file on every open.
/// An incremental edit replays the document and reopens the same fonts and
/// class files each time, so on a large font that copy would land squarely on
/// the keystroke path.
struct SharedFile {
    bytes: FileBytes,
    pos: usize,
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (self.bytes.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&self.bytes[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

impl InputFeatures for SharedFile {
    fn get_size(&mut self) -> tectonic_errors::Result<usize> {
        Ok(self.bytes.len())
    }

    fn try_seek(&mut self, pos: SeekFrom) -> tectonic_errors::Result<u64> {
        let len = self.bytes.len() as i64;
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => len + offset,
            SeekFrom::Current(offset) => self.pos as i64 + offset,
        };

        if target < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot seek before the start of the file",
            )
            .into());
        }

        // Seeking past the end is legal and reads nothing, as with a real file.
        self.pos = (target as usize).min(self.bytes.len());
        Ok(self.pos as u64)
    }
}

/// An [`IoProvider`] that resolves support files through the host.
///
/// Every file the engine reads is cached for the lifetime of the session. That
/// matters more here than it would natively: an incremental edit replays the
/// document from a checkpoint, so the same class and font files are opened
/// again on every keystroke, and each miss would otherwise be a blocking
/// network round trip in the middle of typesetting.
///
/// Cloning shares the cache rather than copying it: the engine reads through
/// this as an `IoProvider` while the renderer reads through it as a
/// `ResourceProvider`, and a font fetched for one must not be fetched again for
/// the other.
#[derive(Default, Clone)]
pub struct HostIo {
    /// Files already resolved, including the misses — a name known not to exist
    /// is worth remembering, since TeX probes many candidates per lookup.
    cache: Rc<RefCell<HashMap<String, Option<FileBytes>>>>,
    /// Names the host was asked for, in first-request order.
    requested: Rc<RefCell<Vec<String>>>,
}

impl HostIo {
    /// Construct an empty support-file provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the cache with a file the host already has, avoiding a round trip.
    pub fn preload(&self, name: impl Into<String>, bytes: impl Into<FileBytes>) {
        self.cache
            .borrow_mut()
            .insert(name.into(), Some(bytes.into()));
    }

    /// Every support-file name the engine has asked for, in request order.
    ///
    /// The host uses this to know what to persist for the next cold start.
    pub fn requested_names(&self) -> Vec<String> {
        self.requested.borrow().clone()
    }

    /// How many distinct support files are resident.
    pub fn cached_count(&self) -> usize {
        self.cache.borrow().values().filter(|v| v.is_some()).count()
    }

    /// Total bytes of support files held in memory.
    pub fn cached_bytes(&self) -> usize {
        self.cache
            .borrow()
            .values()
            .filter_map(|v| v.as_ref())
            .map(|bytes| bytes.len())
            .sum()
    }

    /// Read a support file, going to the host on a miss.
    ///
    /// Shares its cache with the engine's own reads, so the renderer asking for
    /// a font the engine has already loaded costs nothing.
    pub fn read(&self, name: &str) -> Result<Option<FileBytes>, String> {
        self.resolve(name)
    }

    fn resolve(&self, name: &str) -> Result<Option<FileBytes>, String> {
        if let Some(hit) = self.cache.borrow().get(name) {
            return Ok(hit.clone());
        }

        self.requested.borrow_mut().push(name.to_owned());

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
        match self.resolve(name) {
            Ok(Some(bytes)) => OpenResult::Ok(InputHandle::new_read_only(
                name,
                SharedFile { bytes, pos: 0 },
                InputOrigin::Other,
            )),
            Ok(None) => OpenResult::NotAvailable,
            Err(error) => OpenResult::Err(tectonic_errors::anyhow::anyhow!(error)),
        }
    }
}

/// A [`StatusBackend`] that forwards the engine's diagnostics to the host and
/// appends them to a transcript shared with whoever will report them.
///
/// The transcript is shared rather than owned because a status backend lives
/// only as long as one engine operation, and the operation is nested inside the
/// pipeline by the time it runs — there is nowhere to hand a return value back
/// from.
pub struct HostStatus {
    lines: Rc<RefCell<Vec<String>>>,
}

impl HostStatus {
    /// Construct a status backend appending to a shared transcript.
    pub fn new(lines: Rc<RefCell<Vec<String>>>) -> Self {
        Self { lines }
    }
}

impl StatusBackend for HostStatus {
    fn report(
        &mut self,
        kind: MessageKind,
        args: std::fmt::Arguments,
        err: Option<&tectonic_errors::Error>,
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
