use std::{
    fmt,
    io::{BufRead, Write},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use tectonic_preview_protocol::{
    read_request, ClientRequest, ProtocolError, ProtocolLimits, ServerEvent, PROTOCOL_VERSION,
};

use super::{
    DocumentRoot, JsonlFrameSink, LivePreviewWorker, PreviewTuning, PreviewWorkerProcessor,
    PublishedFrame, SourceError, SourceSet, TransportError, WorkerEvent, WorkerFailure,
};

/// Metadata frozen by worker-thread bootstrap before `ready` is emitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewBootstrap {
    /// Stable capability names supported by this server.
    pub capabilities: Vec<String>,
    /// Digest of the exact support bundle/resource universe.
    pub bundle_digest: String,
}

/// Cloneable writer whose each `write` call holds one shared stream lock.
///
/// The implementation turns a partial inner write into `write_all` while the
/// lock is held, preventing worker frames and request-side events from
/// interleaving on protocol stdout.
pub struct SharedJsonlWriter<W>(Arc<Mutex<W>>);

impl<W> Clone for SharedJsonlWriter<W> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<W> SharedJsonlWriter<W> {
    /// Wrap one protocol-only output stream.
    pub fn new(writer: W) -> Self {
        Self(Arc::new(Mutex::new(writer)))
    }

    /// Inspect the underlying writer while holding its lock.
    pub fn inspect<R>(&self, inspect: impl FnOnce(&W) -> R) -> Result<R, PreviewServerError> {
        let writer = self.0.lock().map_err(|_| PreviewServerError::Poisoned)?;
        Ok(inspect(&writer))
    }

    fn into_inner(self) -> Result<W, PreviewServerError> {
        Arc::try_unwrap(self.0)
            .map_err(|_| PreviewServerError::WriterStillShared)?
            .into_inner()
            .map_err(|_| PreviewServerError::Poisoned)
    }
}

impl<W: Write> Write for SharedJsonlWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let mut writer = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("preview output lock poisoned"))?;
        writer.write_all(buffer)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("preview output lock poisoned"))?
            .flush()
    }
}

/// Bootstrap, request, worker, or protocol-output failure.
#[derive(Debug)]
pub enum PreviewServerError {
    /// Input line or schema validation failed.
    Protocol(ProtocolError),
    /// Initial source authority or sizing validation failed.
    Source(SourceError),
    /// Worker lifecycle or communication failed.
    Worker(WorkerFailure),
    /// Worker reported an unrecoverable session failure.
    WorkerFatal(String),
    /// Protocol output failed.
    Transport(TransportError),
    /// Primary path cannot establish a document root.
    InvalidPrimaryPath(String),
    /// Input reader thread panicked.
    ReaderPanicked,
    /// Shared output lock was poisoned.
    Poisoned,
    /// A worker-owned output clone remained after clean shutdown.
    WriterStillShared,
}

impl fmt::Display for PreviewServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(f),
            Self::Source(error) => error.fmt(f),
            Self::Worker(error) => error.fmt(f),
            Self::WorkerFatal(error) => write!(f, "preview worker failed: {error}"),
            Self::Transport(error) => error.fmt(f),
            Self::InvalidPrimaryPath(path) => {
                write!(f, "primary source path {path:?} has no document root")
            }
            Self::ReaderPanicked => write!(f, "preview protocol reader panicked"),
            Self::Poisoned => write!(f, "preview protocol output lock was poisoned"),
            Self::WriterStillShared => write!(f, "preview protocol writer remained shared"),
        }
    }
}

impl std::error::Error for PreviewServerError {}

/// Run the versioned stdio protocol around a worker-thread processor factory.
///
/// The factory runs on the engine worker and must return its frozen `ready`
/// metadata with the processor. Stdout is represented solely by `writer`;
/// callers must route all diagnostics elsewhere.
pub fn serve_preview_jsonl<R, W, P, F, E>(
    mut reader: R,
    writer: W,
    limits: ProtocolLimits,
    tuning: PreviewTuning,
    factory: F,
) -> Result<W, PreviewServerError>
where
    R: BufRead + Send + 'static,
    W: Write + Send + 'static,
    P: PreviewWorkerProcessor<Output = PublishedFrame> + 'static,
    F: FnOnce(SharedJsonlWriter<W>) -> Result<(P, PreviewBootstrap), E> + Send + 'static,
    E: fmt::Display,
{
    let shared = SharedJsonlWriter::new(writer);
    let worker_writer = shared.clone();
    let (mut worker, bootstrap) =
        LivePreviewWorker::spawn_uninitialized(tuning, move || factory(worker_writer))
            .map_err(PreviewServerError::Worker)?;

    write_event(
        &shared,
        limits,
        &ServerEvent::Ready {
            protocol_version: PROTOCOL_VERSION,
            capabilities: bootstrap.capabilities,
            bundle_digest: bootstrap.bundle_digest,
        },
    )?;

    let initialization = match read_request(&mut reader, limits) {
        Ok(request) => request,
        Err(protocol) => {
            let error = PreviewServerError::Protocol(protocol);
            let _ = write_fatal(&shared, limits, &error.to_string());
            let _ = worker.shutdown();
            return Err(error);
        }
    };
    let ClientRequest::Initialize {
        primary_path,
        primary_text,
        children,
        ..
    } = initialization
    else {
        let error = PreviewServerError::Protocol(ProtocolError::InvalidRequest(
            "first request after ready must be initialize".into(),
        ));
        let _ = write_fatal(&shared, limits, &error.to_string());
        let _ = worker.shutdown();
        return Err(error);
    };
    let sources = (|| {
        let primary = absolute_primary(&primary_path)?;
        let root_path = primary
            .parent()
            .ok_or_else(|| PreviewServerError::InvalidPrimaryPath(primary_path.clone()))?;
        let root = DocumentRoot::new(root_path).map_err(PreviewServerError::Source)?;
        SourceSet::initialize(
            root,
            primary.to_string_lossy().as_ref(),
            primary_text,
            children,
            limits,
        )
        .map_err(PreviewServerError::Source)
    })();
    let sources = match sources {
        Ok(sources) => sources,
        Err(error) => {
            let _ = write_fatal(&shared, limits, &error.to_string());
            let _ = worker.shutdown();
            return Err(error);
        }
    };
    if let Err(worker_error) = worker.initialize(sources) {
        let error = PreviewServerError::Worker(worker_error);
        let _ = write_fatal(&shared, limits, &error.to_string());
        let _ = worker.shutdown();
        return Err(error);
    }

    let (request_tx, request_rx) = mpsc::channel();
    let reader_thread = thread::spawn(move || loop {
        let request = read_request(&mut reader, limits);
        let stop = matches!(
            request,
            Ok(ClientRequest::Shutdown {}) | Err(ProtocolError::EndOfStream)
        );
        if request_tx.send(request).is_err() || stop {
            break;
        }
    });

    let mut shutdown = false;
    let mut fatal = None;
    while !shutdown && fatal.is_none() {
        while let Ok(request) = request_rx.try_recv() {
            match request {
                Ok(ClientRequest::Replace { path, text }) => {
                    if let Err(error) = worker.replace(path, text) {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::Viewport { first, last }) => {
                    if let Err(error) = worker.viewport(first, last) {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::ForwardSearch {
                    query_id,
                    path,
                    line,
                }) => {
                    if let Err(error) = worker.forward_search(query_id, path, line) {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::BackwardSearch {
                    query_id,
                    page_index,
                    h_bp,
                    v_bp,
                }) => {
                    if let Err(error) = worker.backward_search(query_id, page_index, h_bp, v_bp) {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::Resync {}) => {
                    if let Err(error) = worker.resync() {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::Cancel {}) => {
                    if let Err(error) = worker.cancel() {
                        fatal = Some(PreviewServerError::Worker(error));
                        break;
                    }
                }
                Ok(ClientRequest::Painted { .. }) => {}
                Ok(ClientRequest::Shutdown {}) | Err(ProtocolError::EndOfStream) => {
                    shutdown = true;
                    break;
                }
                Ok(ClientRequest::Initialize { .. }) => {
                    fatal = Some(PreviewServerError::Protocol(ProtocolError::InvalidRequest(
                        "initialize may appear exactly once".into(),
                    )));
                    break;
                }
                Err(error) => {
                    fatal = Some(PreviewServerError::Protocol(error));
                    break;
                }
            }
        }
        if shutdown || fatal.is_some() {
            break;
        }

        match worker.recv_timeout(Duration::from_millis(10)) {
            Ok(WorkerEvent::Update { .. }) => {
                // The processor's atomic sink already emitted this frame.
            }
            Ok(WorkerEvent::Cancelled {
                applied_request_seq,
                source_revision,
            }) => write_event(
                &shared,
                limits,
                &ServerEvent::Cancelled {
                    applied_request_seq,
                    source_revision,
                },
            )?,
            Ok(WorkerEvent::Stopped {
                applied_request_seq,
                source_revision,
            }) => write_event(
                &shared,
                limits,
                &ServerEvent::Stopped {
                    applied_request_seq,
                    source_revision,
                },
            )?,
            Ok(WorkerEvent::Rejected {
                request_seq,
                source_revision,
                reason,
            }) => write_event(
                &shared,
                limits,
                &ServerEvent::Rejected {
                    request_seq,
                    source_revision,
                    reason,
                },
            )?,
            Ok(WorkerEvent::ForwardSearchResult { query_id, result }) => write_event(
                &shared,
                limits,
                &ServerEvent::ForwardSearchResult { query_id, result },
            )?,
            Ok(WorkerEvent::BackwardSearchResult { query_id, result }) => write_event(
                &shared,
                limits,
                &ServerEvent::BackwardSearchResult { query_id, result },
            )?,
            Ok(WorkerEvent::Aborted {
                applied_request_seq,
                source_revision,
                reason,
            }) => write_event(
                &shared,
                limits,
                &ServerEvent::Aborted {
                    applied_request_seq,
                    source_revision,
                    log_tail: reason,
                },
            )?,
            Ok(WorkerEvent::Fatal(reason)) => {
                fatal = Some(PreviewServerError::WorkerFatal(reason));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                fatal = Some(PreviewServerError::Worker(WorkerFailure::Closed));
            }
        }
    }

    if let Some(error) = &fatal {
        let _ = write_fatal(&shared, limits, &error.to_string());
    }
    worker.shutdown().map_err(PreviewServerError::Worker)?;
    if shutdown && reader_thread.join().is_err() {
        return Err(PreviewServerError::ReaderPanicked);
    }
    if let Some(error) = fatal {
        return Err(error);
    }
    shared.into_inner()
}

fn absolute_primary(path: &str) -> Result<PathBuf, PreviewServerError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| PreviewServerError::Protocol(ProtocolError::Io(error)))
    }
}

fn write_event<W: Write>(
    writer: &SharedJsonlWriter<W>,
    limits: ProtocolLimits,
    event: &ServerEvent,
) -> Result<(), PreviewServerError> {
    JsonlFrameSink::new(writer.clone(), limits)
        .write_event(event)
        .map_err(PreviewServerError::Transport)
}

fn write_fatal<W: Write>(
    writer: &SharedJsonlWriter<W>,
    limits: ProtocolLimits,
    reason: &str,
) -> Result<(), PreviewServerError> {
    write_event(
        writer,
        limits,
        &ServerEvent::Fatal {
            reason: reason.into(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::io;

    use tectonic_preview_protocol::{
        encode_event, FrameId, FramePhase, OperationMetrics, PageSize, RequestSeq, UpdateOutcome,
    };

    use super::*;
    use crate::{AcceptedSourceSnapshot, WorkerRun};
    use tectonic_engine_xetex::EngineCancellation;

    struct WireProcessor<W> {
        writer: SharedJsonlWriter<W>,
        limits: ProtocolLimits,
        next_frame: FrameId,
    }

    #[derive(Clone, Default)]
    struct ProbeWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for ProbeWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<W: Write> WireProcessor<W> {
        fn emit(
            &mut self,
            snapshot: &AcceptedSourceSnapshot,
            sequence: RequestSeq,
        ) -> PublishedFrame {
            self.next_frame = self.next_frame.checked_next().unwrap();
            let event = ServerEvent::Update {
                frame_id: self.next_frame,
                source_revision: snapshot.revision(),
                applied_request_seq: sequence,
                phase: FramePhase::Foreground,
                outcome: UpdateOutcome::Complete,
                page_count: 0,
                changed: vec![],
                removed: vec![],
                pages_svg: vec![],
                page_sizes: Vec::<PageSize>::new(),
                unsupported_pages: vec![],
                metrics: OperationMetrics::default(),
                tail_pending: false,
            };
            let bytes = encode_event(&event, self.limits).unwrap();
            self.writer.write_all(&bytes).unwrap();
            PublishedFrame {
                frame_id: self.next_frame,
                outcome: UpdateOutcome::Complete,
                tail_pending: false,
            }
        }
    }

    impl<W: Write + 'static> PreviewWorkerProcessor for WireProcessor<W> {
        type Output = PublishedFrame;
        type Error = Infallible;

        fn compile(
            &mut self,
            snapshot: AcceptedSourceSnapshot,
            applied_request_seq: RequestSeq,
            _viewport: Option<(usize, usize)>,
            _cancellation: &EngineCancellation,
        ) -> super::super::WorkerRun<Self::Output, Self::Error> {
            WorkerRun::Complete(self.emit(&snapshot, applied_request_seq))
        }

        fn resync(
            &mut self,
            snapshot: AcceptedSourceSnapshot,
            applied_request_seq: RequestSeq,
        ) -> Result<Self::Output, Self::Error> {
            Ok(self.emit(&snapshot, applied_request_seq))
        }
    }

    #[test]
    fn ready_precedes_initialized_frame_and_counters_are_strings() {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("main.tex");
        let input = format!(
            "{{\"type\":\"initialize\",\"protocol_version\":1,\"primary_path\":{},\"primary_text\":\"A\\n\\\\bye\\n\",\"children\":[]}}\n{{\"type\":\"shutdown\"}}\n",
            serde_json::to_string(primary.to_string_lossy().as_ref()).unwrap()
        );
        let limits = ProtocolLimits::default();
        let output = serve_preview_jsonl(
            std::io::Cursor::new(input.into_bytes()),
            Vec::new(),
            limits,
            PreviewTuning::default(),
            move |writer| {
                Ok::<_, Infallible>((
                    WireProcessor {
                        writer,
                        limits,
                        next_frame: FrameId::ZERO,
                    },
                    PreviewBootstrap {
                        capabilities: vec!["svg".into()],
                        bundle_digest: "abc".into(),
                    },
                ))
            },
        )
        .unwrap();

        let lines = output
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let ready: serde_json::Value = serde_json::from_slice(lines[0]).unwrap();
        let update: serde_json::Value = serde_json::from_slice(lines[1]).unwrap();
        assert_eq!(ready["type"], "ready");
        assert_eq!(update["type"], "update");
        assert_eq!(update["frame_id"], "1");
        assert_eq!(update["source_revision"], "0");
        assert_eq!(update["applied_request_seq"], "0");
    }

    #[test]
    fn request_before_initialize_is_fatal_after_ready() {
        let limits = ProtocolLimits::default();
        let probe = ProbeWriter::default();
        let output = probe.clone();
        let result = serve_preview_jsonl(
            std::io::Cursor::new(
                b"{\"type\":\"replace\",\"path\":null,\"text\":\"bad\"}\n".to_vec(),
            ),
            output,
            limits,
            PreviewTuning::default(),
            move |writer| {
                Ok::<_, Infallible>((
                    WireProcessor {
                        writer,
                        limits,
                        next_frame: FrameId::ZERO,
                    },
                    PreviewBootstrap {
                        capabilities: vec!["svg".into()],
                        bundle_digest: "abc".into(),
                    },
                ))
            },
        );
        assert!(matches!(result, Err(PreviewServerError::Protocol(_))));

        let wire = probe.0.lock().unwrap().clone();
        let events = wire
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], "ready");
        assert_eq!(events[1]["type"], "fatal");
    }
}
