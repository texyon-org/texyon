//! What the engine reaches the outside world through, lent for each operation.

use std::borrow::Cow;

use texyon_bridge_core::{BridgeHost, CoreBridgeState, Diagnostic};
use texyon_io_base::OutputHandle;

use std::collections::BTreeSet;

use crate::files::io::LineScratch;
use crate::preview::replay::adapter::SnapshotSources;
use crate::preview::replay::source::{SourcePath, SourceSnapshot};
use crate::preview::session::FenceController;

/// The engine's environment: its owner lends it to each operation, apart from
/// the typesetting state, so checkpoints and rewinds leave it alone.
pub struct EngineEnv<'h> {
    pub(crate) host: BridgeHost<'h>,
    outputs: OutputFiles,
    /// The preview's state while one of its operations runs.
    operation: Option<PreviewOperation>,
    pub(crate) line_scratch: LineScratch,
}

/// `bytes` as bridge-core saw them through a C string: empty when they hold a NUL.
fn c_view(bytes: &[u8]) -> Cow<'_, str> {
    if bytes.contains(&0) {
        Cow::Borrowed("")
    } else {
        String::from_utf8_lossy(bytes)
    }
}

/// What a preview operation keeps while it runs: the sources it reads and the
/// fences it captures.
pub(crate) struct PreviewOperation {
    pub(crate) sources: SnapshotSources,
    pub(crate) fences: Option<FenceController>,
}

/// Bytes a buffered output collects before it writes them out.
const OUTPUT_BUFFER: usize = 8 * 1024;

/// An output written through a buffer of its own.
struct BufferedOutput {
    handle: OutputHandle,
    buf: Vec<u8>,
}

impl BufferedOutput {
    fn new(handle: OutputHandle) -> Self {
        BufferedOutput {
            handle,
            buf: Vec::with_capacity(OUTPUT_BUFFER),
        }
    }
}

/// A `\openout` stream number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WriteStream(u8);

impl WriteStream {
    /// Streams 0 to 15 have files, the others write to the terminal and log.
    pub(crate) fn new(stream: usize) -> Option<Self> {
        u8::try_from(stream)
            .ok()
            .filter(|&s| s < 16)
            .map(WriteStream)
    }
}

/// The files the engine writes.
#[derive(Default)]
struct OutputFiles {
    terminal: Option<BufferedOutput>,
    log: Option<BufferedOutput>,
    write: [Option<BufferedOutput>; 16],
    xdv: Option<OutputHandle>,
    synctex: Option<OutputHandle>,
}

impl<'h> EngineEnv<'h> {
    /// An environment over `host`.
    pub fn new(host: BridgeHost<'h>) -> Self {
        EngineEnv {
            host,
            outputs: OutputFiles::default(),
            operation: None,
            line_scratch: LineScratch::default(),
        }
    }

    /// Writes `out`'s buffer to its handle. Returns true on failure.
    fn drain(host: &mut BridgeHost<'h>, out: &mut BufferedOutput) -> bool {
        if out.buf.is_empty() {
            return false;
        }
        let failed = host.write_output(&mut out.handle, &out.buf);
        out.buf.clear();
        failed
    }

    fn push(host: &mut BridgeHost<'h>, out: &mut Option<BufferedOutput>, bytes: &[u8]) {
        if let Some(out) = out {
            out.buf.extend_from_slice(bytes);
            if out.buf.len() >= OUTPUT_BUFFER {
                Self::drain(host, out);
            }
        }
    }

    /// Writes out and closes `out`. Returns true on failure.
    fn close(host: &mut BridgeHost<'h>, out: Option<BufferedOutput>) -> bool {
        let Some(mut out) = out else {
            return false;
        };
        let failed = Self::drain(host, &mut out);
        host.close_output(out.handle) || failed
    }

    /// Serves `snapshot` to the engine until [`Self::take_sources`]. `written`
    /// lists the files the engine has written so far.
    pub(crate) fn install_sources(
        &mut self,
        snapshot: SourceSnapshot,
        written: BTreeSet<SourcePath>,
    ) {
        self.operation = Some(PreviewOperation {
            sources: SnapshotSources::new(snapshot, written),
            fences: None,
        });
    }

    /// Ends the preview operation, returning the files the engine has written.
    pub(crate) fn take_sources(&mut self) -> BTreeSet<SourcePath> {
        self.operation
            .take()
            .map(|operation| operation.sources.into_written())
            .unwrap_or_default()
    }

    /// The installed sources, if an operation is serving a snapshot.
    pub(crate) fn sources(&mut self) -> Option<&mut SnapshotSources> {
        self.operation
            .as_mut()
            .map(|operation| &mut operation.sources)
    }

    /// The running preview operation.
    pub(crate) fn operation(&mut self) -> Option<&mut PreviewOperation> {
        self.operation.as_mut()
    }

    /// Runs `f` with a bridge state over the host, for code that names files
    /// by id. Files it leaves open are dropped with the state.
    pub(crate) fn bridge_state<R>(
        &mut self,
        f: impl FnOnce(&mut CoreBridgeState<'_, '_>) -> R,
    ) -> R {
        // C reports straight to the host, after what the terminal holds.
        self.flush_terminal();
        f(&mut CoreBridgeState::new(&mut self.host))
    }

    /// Whether `dir` exists in the sources or the document, so a file can be created in it.
    pub(crate) fn directory_exists(&mut self, dir: &[u8]) -> bool {
        let dir = String::from_utf8_lossy(dir);
        self.sources()
            .is_some_and(|sources| sources.holds_directory(&dir))
            || self.host.directory_exists(&dir)
    }

    /// Reports a finished diagnostic to the host.
    pub(crate) fn report(&mut self, diag: Diagnostic) {
        self.flush_terminal();
        self.host.finish_diagnostic(diag);
    }

    /// Issues `text` as a warning, empty when it holds a NUL.
    pub(crate) fn warn(&mut self, text: &str) {
        self.flush_terminal();
        self.host.issue_warning(&c_view(text.as_bytes()));
    }

    /// Issues `text` as an error, empty when it holds a NUL.
    pub(crate) fn error(&mut self, text: &str) {
        self.flush_terminal();
        self.host.issue_error(&c_view(text.as_bytes()));
    }

    /// Asks the host to run `cmd`.
    pub(crate) fn shell_escape(&mut self, cmd: &[u16]) {
        match String::from_utf16(cmd) {
            Ok(cmd) => {
                self.host.shell_escape(&cmd);
            }
            Err(err) => self
                .host
                .issue_error(&format!("command was not valid UTF-16: {err}")),
        }
    }

    /// The MD5 digest of file `path`, `None` if it cannot be read.
    pub(crate) fn file_md5(&mut self, path: &[u8]) -> Option<[u8; 16]> {
        let mut digest = [0; 16];
        let failed = self.host.get_file_md5(&c_view(path), &mut digest);
        (!failed).then_some(digest)
    }

    /// Opens output `path`, outside the files the env keeps.
    pub(crate) fn open(&mut self, path: &[u8], is_gz: bool) -> Option<OutputHandle> {
        let path = String::from_utf8_lossy(path);
        if let Some(operation) = &mut self.operation {
            operation.sources.record_written(&path);
        }
        self.host.open_output(&path, is_gz)
    }

    /// Opens the terminal, returning whether it is open.
    pub(crate) fn open_terminal(&mut self) -> bool {
        self.outputs.terminal = self.host.open_stdout().map(BufferedOutput::new);
        self.outputs.terminal.is_some()
    }

    #[inline]
    pub(crate) fn terminal_put(&mut self, bytes: &[u8]) {
        Self::push(&mut self.host, &mut self.outputs.terminal, bytes);
    }

    /// Brings the terminal up to date, as TeX's `update_terminal`.
    pub(crate) fn flush_terminal(&mut self) {
        if let Some(out) = &mut self.outputs.terminal {
            Self::drain(&mut self.host, out);
            self.host.flush_output(&mut out.handle);
        }
    }

    /// Opens the transcript at `path`, returning whether it is open.
    pub(crate) fn open_log(&mut self, path: &[u8]) -> bool {
        self.outputs.log = self.open(path, false).map(BufferedOutput::new);
        self.outputs.log.is_some()
    }

    #[inline]
    pub(crate) fn log_put(&mut self, bytes: &[u8]) {
        Self::push(&mut self.host, &mut self.outputs.log, bytes);
    }

    /// Closes the transcript. Returns true on failure.
    pub(crate) fn close_log(&mut self) -> bool {
        Self::close(&mut self.host, self.outputs.log.take())
    }

    /// Opens `\openout` stream `stream` at `path`, returning whether it is open.
    pub(crate) fn open_write(&mut self, stream: WriteStream, path: &[u8]) -> bool {
        let opened = self.open(path, false).map(BufferedOutput::new);
        let slot = &mut self.outputs.write[usize::from(stream.0)];
        *slot = opened;
        slot.is_some()
    }

    pub(crate) fn write_is_open(&self, stream: WriteStream) -> bool {
        self.outputs.write[usize::from(stream.0)].is_some()
    }

    #[inline]
    pub(crate) fn write_put(&mut self, stream: WriteStream, bytes: &[u8]) {
        let slot = &mut self.outputs.write[usize::from(stream.0)];
        Self::push(&mut self.host, slot, bytes);
    }

    /// Closes `\openout` stream `stream`. Returns true on failure.
    pub(crate) fn close_write(&mut self, stream: WriteStream) -> bool {
        let out = self.outputs.write[usize::from(stream.0)].take();
        Self::close(&mut self.host, out)
    }

    /// Opens the XDV file at `path`, returning whether it is open.
    pub(crate) fn open_xdv(&mut self, path: &[u8]) -> bool {
        self.outputs.xdv = self.open(path, false);
        self.outputs.xdv.is_some()
    }

    /// Writes `bytes` to the XDV file, returning whether they were written.
    pub(crate) fn write_xdv(&mut self, bytes: &[u8]) -> bool {
        match &mut self.outputs.xdv {
            Some(handle) => !self.host.write_output(handle, bytes),
            None => false,
        }
    }

    /// Closes the XDV file. Returns true on failure.
    pub(crate) fn close_xdv(&mut self) -> bool {
        match self.outputs.xdv.take() {
            Some(handle) => self.host.close_output(handle),
            None => false,
        }
    }

    /// Opens the gzipped SyncTeX file at `path`, returning whether it is open.
    pub(crate) fn open_synctex(&mut self, path: &[u8]) -> bool {
        self.outputs.synctex = self.open(path, true);
        self.outputs.synctex.is_some()
    }

    /// Writes `bytes` to the SyncTeX file, returning whether they were written.
    pub(crate) fn write_synctex(&mut self, bytes: &[u8]) -> bool {
        match &mut self.outputs.synctex {
            Some(handle) => !self.host.write_output(handle, bytes),
            None => false,
        }
    }

    pub(crate) fn close_synctex(&mut self) {
        if let Some(handle) = self.outputs.synctex.take() {
            self.host.close_output(handle);
        }
    }

    /// Forgets the open files, as a rewind reopens the ones it needs. The
    /// terminal stays.
    pub(crate) fn discard_files(&mut self) {
        self.flush_outputs();
        let terminal = self.outputs.terminal.take();
        self.outputs = OutputFiles {
            terminal,
            ..OutputFiles::default()
        };
    }

    /// Ends an operation: writes out and drops every output, which publishes
    /// what the engine wrote to hosts that keep files in memory.
    pub(crate) fn end_operation(&mut self) {
        self.flush_outputs();
        self.outputs = OutputFiles::default();
    }

    /// Writes out every buffered output, so the host sees all the engine wrote.
    pub fn flush_outputs(&mut self) {
        let OutputFiles {
            terminal,
            log,
            write,
            ..
        } = &mut self.outputs;
        for out in [terminal, log]
            .into_iter()
            .chain(write.iter_mut())
            .flatten()
        {
            Self::drain(&mut self.host, out);
        }
    }
}

/// An env over empty in-memory I/O that reports nothing, for unit tests.
#[cfg(test)]
pub(crate) fn test_env() -> EngineEnv<'static> {
    use texyon_bridge_core::{Held, MinimalDriver, SecuritySettings};
    use texyon_io_base::memory::MemoryIo;
    use texyon_status_base::NoopStatusBackend;

    EngineEnv::new(BridgeHost::new(
        Held::Owned(Box::new(MinimalDriver::new(MemoryIo::new(false)))),
        Held::Owned(Box::new(NoopStatusBackend::default())),
        SecuritySettings::default(),
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fmt::Arguments;
    use std::io::{self, Write};
    use std::rc::Rc;

    use texyon_bridge_core::{Held, MinimalDriver, SecuritySettings};
    use texyon_errors::Error;
    use texyon_io_base::{IoProvider, OpenResult};
    use texyon_status_base::{MessageKind, StatusBackend};

    use super::*;

    type Bytes = Rc<RefCell<Vec<u8>>>;

    struct Sink(Bytes);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Outputs whose bytes are visible as soon as they are written.
    #[derive(Default)]
    struct SinkIo {
        stdout: Bytes,
        file: Bytes,
    }

    impl IoProvider for SinkIo {
        fn output_open_name(&mut self, name: &str) -> OpenResult<OutputHandle> {
            OpenResult::Ok(OutputHandle::new(name, Sink(self.file.clone())))
        }

        fn output_open_stdout(&mut self) -> OpenResult<OutputHandle> {
            OpenResult::Ok(OutputHandle::new("", Sink(self.stdout.clone())))
        }
    }

    /// Records what the terminal held when each message arrived.
    struct TerminalAtReport {
        stdout: Bytes,
        seen: Vec<Vec<u8>>,
    }

    impl StatusBackend for TerminalAtReport {
        fn report(&mut self, _: MessageKind, _: Arguments, _: Option<&Error>) {
            self.seen.push(self.stdout.borrow().clone());
        }

        fn dump_error_logs(&mut self, _: &[u8]) {}
    }

    #[test]
    fn a_warning_follows_the_terminal_output_before_it() {
        let io = SinkIo::default();
        let stdout = io.stdout.clone();
        let mut hooks = MinimalDriver::new(io);
        let mut status = TerminalAtReport {
            stdout: stdout.clone(),
            seen: Vec::new(),
        };
        {
            let host = BridgeHost::new(
                Held::Borrowed(&mut hooks),
                Held::Borrowed(&mut status),
                SecuritySettings::default(),
            );
            let mut env = EngineEnv::new(host);
            assert!(env.open_terminal());
            env.terminal_put(b"(main.tex");
            assert!(stdout.borrow().is_empty());
            env.warn("late");
        }
        assert_eq!(status.seen, [b"(main.tex".to_vec()]);
    }

    #[test]
    fn the_log_reaches_its_file_when_flushed_or_closed() {
        let io = SinkIo::default();
        let file = io.file.clone();
        let mut env = EngineEnv::new(BridgeHost::new(
            Held::Owned(Box::new(MinimalDriver::new(io))),
            Held::Owned(Box::new(texyon_status_base::NoopStatusBackend::default())),
            SecuritySettings::default(),
        ));
        assert!(env.open_log(b"main.log"));
        env.log_put(b"abc");
        assert_eq!(*file.borrow(), b"");
        env.flush_outputs();
        assert_eq!(*file.borrow(), b"abc");
        env.log_put(b"def");
        assert!(!env.close_log());
        assert_eq!(*file.borrow(), b"abcdef");
    }
}
