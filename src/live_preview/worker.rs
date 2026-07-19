use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::{
    AcceptedSourceSnapshot, ApplyDisposition, RequestClock, RequestSeq, SourceRevision, SourceSet,
};
use crate::engines::tex::EngineCancellation;

/// Scheduling and checkpoint tuning fixed for one worker lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewTuning {
    /// Delay before a newer request cancels instrumented stale work.
    pub supersede_grace_ms: u64,
    /// Reserved tail-settle idle delay.
    pub tail_idle_ms: u64,
    /// Soft checkpoint-ladder byte target.
    pub ladder_bytes: usize,
    /// Enable expensive rewind verification.
    pub verify_rewinds: bool,
}

impl Default for PreviewTuning {
    fn default() -> Self {
        Self {
            supersede_grace_ms: 500,
            tail_idle_ms: 200,
            ladder_bytes: 64 * 1024 * 1024,
            verify_rewinds: false,
        }
    }
}

/// Result of one processor operation.
#[derive(Debug)]
pub enum WorkerRun<T, E> {
    /// Coherent output completed.
    Complete(T),
    /// Cooperative cancellation restored processor state.
    Cancelled,
    /// Recoverable operation failure retained last-good state.
    Aborted(E),
}

/// Worker-local engine/render implementation constructed on the worker thread.
pub trait PreviewWorkerProcessor {
    /// Complete coherent output sent to the transport side.
    type Output: Send + 'static;
    /// Recoverable or fatal processor error.
    type Error: fmt::Display;

    /// Compile one immutable accepted snapshot.
    fn compile(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        applied_request_seq: RequestSeq,
        viewport: Option<(usize, usize)>,
        cancellation: &EngineCancellation,
    ) -> WorkerRun<Self::Output, Self::Error>;

    /// Reconstruct a full retained-state payload without recompiling.
    fn resync(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        applied_request_seq: RequestSeq,
    ) -> Result<Self::Output, Self::Error>;

    /// Release resident processor resources on their owning thread.
    fn shutdown(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Observable worker state transition.
#[derive(Debug)]
pub enum WorkerEvent<T> {
    /// Coherent initial, edit, or resync output.
    Update {
        /// Latest request applied before this output.
        applied_request_seq: RequestSeq,
        /// Accepted source revision carried by this output.
        source_revision: SourceRevision,
        /// Processor-owned coherent payload.
        output: T,
        /// Whether this is a no-compile resync payload.
        resync: bool,
    },
    /// Stale instrumented work was cooperatively cancelled.
    Cancelled {
        /// Request sequence of the cancelled compile snapshot.
        applied_request_seq: RequestSeq,
        /// Source revision of the cancelled compile snapshot.
        source_revision: SourceRevision,
    },
    /// Durable user stop is active.
    Stopped {
        /// Latest request applied by the worker.
        applied_request_seq: RequestSeq,
        /// Latest accepted source revision.
        source_revision: SourceRevision,
    },
    /// A mutating request failed validation but advanced applied accounting.
    Rejected {
        /// Sequence assigned to the rejected request.
        request_seq: RequestSeq,
        /// Unchanged accepted source revision.
        source_revision: SourceRevision,
        /// Validation failure.
        reason: String,
    },
    /// Processor operation failed while last-good state was retained.
    Aborted {
        /// Latest request applied by the worker.
        applied_request_seq: RequestSeq,
        /// Accepted source revision attempted by the operation.
        source_revision: SourceRevision,
        /// Bounded processor failure text.
        reason: String,
    },
    /// Worker cannot continue safely.
    Fatal(String),
}

/// Failure to spawn, communicate with, or join the worker.
#[derive(Debug)]
pub enum WorkerFailure {
    /// Thread creation failed.
    Spawn(std::io::Error),
    /// Worker command channel is closed.
    Closed,
    /// Worker-thread bundle/format/renderer bootstrap failed.
    Bootstrap(String),
    /// A checked request sequence was exhausted.
    RequestSequenceExhausted,
    /// Worker thread panicked.
    Panicked,
}

impl fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(f, "could not spawn preview worker: {error}"),
            Self::Closed => write!(f, "preview worker channel is closed"),
            Self::Bootstrap(error) => write!(f, "preview worker bootstrap failed: {error}"),
            Self::RequestSequenceExhausted => write!(f, "request sequence counter exhausted"),
            Self::Panicked => write!(f, "preview worker panicked"),
        }
    }
}

impl std::error::Error for WorkerFailure {}

enum Command {
    Initialize(SourceSet),
    Replace {
        sequence: RequestSeq,
        path: Option<String>,
        text: String,
    },
    Viewport(usize, usize),
    Resync,
    Cancel,
    Shutdown,
}

enum TimerCommand {
    Arm {
        sequence: RequestSeq,
        deadline: Instant,
    },
    Stop,
}

/// Request-side handle for one 32 MiB resident engine worker.
pub struct LivePreviewWorker<T> {
    commands: Sender<Command>,
    events: Receiver<WorkerEvent<T>>,
    timer_commands: Sender<TimerCommand>,
    cancellation: EngineCancellation,
    announced: Arc<AtomicU64>,
    applied: Arc<AtomicU64>,
    durable_stop: Arc<AtomicBool>,
    worker_done: Arc<AtomicBool>,
    supersede_grace: Duration,
    next_request: RequestSeq,
    worker: Option<JoinHandle<()>>,
    timer: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> LivePreviewWorker<T> {
    /// Spawn a worker and immediately compile the validated revision-zero snapshot.
    ///
    /// `factory` runs on the worker thread, allowing it to construct processor
    /// values that intentionally are not `Send`, such as resident XeTeX state.
    pub fn spawn<P, F>(
        sources: SourceSet,
        tuning: PreviewTuning,
        factory: F,
    ) -> Result<Self, WorkerFailure>
    where
        P: PreviewWorkerProcessor<Output = T> + 'static,
        F: FnOnce() -> P + Send + 'static,
    {
        let (worker, ()) = Self::spawn_inner(tuning, Some(sources), move || {
            Ok::<_, std::convert::Infallible>((factory(), ()))
        })?;
        Ok(worker)
    }

    /// Bootstrap processor resources on the worker, then wait for initialization.
    ///
    /// The returned bootstrap value is available before callers emit `ready`.
    /// No compile begins until [`initialize`](Self::initialize) supplies the
    /// complete validated revision-zero source snapshot.
    pub fn spawn_uninitialized<P, F, E, D>(
        tuning: PreviewTuning,
        factory: F,
    ) -> Result<(Self, D), WorkerFailure>
    where
        P: PreviewWorkerProcessor<Output = T> + 'static,
        F: FnOnce() -> Result<(P, D), E> + Send + 'static,
        E: fmt::Display,
        D: Send + 'static,
    {
        Self::spawn_inner(tuning, None, factory)
    }

    fn spawn_inner<P, F, E, D>(
        tuning: PreviewTuning,
        sources: Option<SourceSet>,
        factory: F,
    ) -> Result<(Self, D), WorkerFailure>
    where
        P: PreviewWorkerProcessor<Output = T> + 'static,
        F: FnOnce() -> Result<(P, D), E> + Send + 'static,
        E: fmt::Display,
        D: Send + 'static,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (timer_tx, timer_rx) = mpsc::channel();
        let (bootstrap_tx, bootstrap_rx) = mpsc::sync_channel(1);
        let cancellation = EngineCancellation::new();
        let announced = Arc::new(AtomicU64::new(0));
        let applied = Arc::new(AtomicU64::new(0));
        let durable_stop = Arc::new(AtomicBool::new(false));
        let worker_done = Arc::new(AtomicBool::new(false));

        let worker_cancellation = cancellation.clone();
        let worker_announced = Arc::clone(&announced);
        let worker_applied = Arc::clone(&applied);
        let worker_stop = Arc::clone(&durable_stop);
        let worker_finished = Arc::clone(&worker_done);
        let worker = thread::Builder::new()
            .name("tectonic-preview-engine".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let (processor, bootstrap) = match factory() {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = bootstrap_tx.send(Err(error.to_string()));
                        worker_finished.store(true, Ordering::Release);
                        return;
                    }
                };
                if bootstrap_tx.send(Ok(bootstrap)).is_err() {
                    worker_finished.store(true, Ordering::Release);
                    return;
                }
                run_worker(
                    processor,
                    sources,
                    command_rx,
                    event_tx,
                    worker_cancellation,
                    worker_announced,
                    worker_applied,
                    worker_stop,
                );
                worker_finished.store(true, Ordering::Release);
            })
            .map_err(WorkerFailure::Spawn)?;

        let bootstrap = match bootstrap_rx.recv() {
            Ok(Ok(bootstrap)) => bootstrap,
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(WorkerFailure::Bootstrap(error));
            }
            Err(_) => {
                let _ = worker.join();
                return Err(WorkerFailure::Panicked);
            }
        };

        let timer_cancellation = cancellation.clone();
        let timer_applied = Arc::clone(&applied);
        let timer = thread::Builder::new()
            .name("tectonic-preview-cancel".into())
            .spawn(move || run_timer(timer_rx, timer_cancellation, timer_applied))
            .map_err(WorkerFailure::Spawn)?;

        let worker = Self {
            commands: command_tx,
            events: event_rx,
            timer_commands: timer_tx,
            cancellation,
            announced,
            applied,
            durable_stop,
            worker_done,
            supersede_grace: Duration::from_millis(tuning.supersede_grace_ms),
            next_request: RequestSeq::ZERO,
            worker: Some(worker),
            timer: Some(timer),
        };
        let _ = tuning.tail_idle_ms;
        let _ = tuning.ladder_bytes;
        let _ = tuning.verify_rewinds;
        Ok((worker, bootstrap))
    }

    /// Supply the one complete initialization snapshot.
    pub fn initialize(&self, sources: SourceSet) -> Result<(), WorkerFailure> {
        self.commands
            .send(Command::Initialize(sources))
            .map_err(|_| WorkerFailure::Closed)
    }

    /// Receive the next worker event, waiting up to `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<WorkerEvent<T>, RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }

    /// Announce and enqueue one complete source replacement.
    pub fn replace(
        &mut self,
        path: Option<String>,
        text: String,
    ) -> Result<RequestSeq, WorkerFailure> {
        let sequence = self
            .next_request
            .checked_next()
            .map_err(|_| WorkerFailure::RequestSequenceExhausted)?;
        self.next_request = sequence;
        self.announced.store(sequence.get(), Ordering::Release);
        self.commands
            .send(Command::Replace {
                sequence,
                path,
                text,
            })
            .map_err(|_| WorkerFailure::Closed)?;
        self.timer_commands
            .send(TimerCommand::Arm {
                sequence,
                deadline: Instant::now() + self.supersede_grace,
            })
            .map_err(|_| WorkerFailure::Closed)?;
        Ok(sequence)
    }

    /// Update the true visible range without entering request accounting.
    pub fn viewport(&self, first: usize, last: usize) -> Result<(), WorkerFailure> {
        self.commands
            .send(Command::Viewport(first, last))
            .map_err(|_| WorkerFailure::Closed)
    }

    /// Request a full retained-state payload without recompiling.
    pub fn resync(&self) -> Result<(), WorkerFailure> {
        self.commands
            .send(Command::Resync)
            .map_err(|_| WorkerFailure::Closed)
    }

    /// Durably cancel current and future compilation until an accepted edit.
    pub fn cancel(&self) -> Result<(), WorkerFailure> {
        self.durable_stop.store(true, Ordering::Release);
        self.cancellation.request();
        self.commands
            .send(Command::Cancel)
            .map_err(|_| WorkerFailure::Closed)
    }

    /// Latest worker-applied request sequence.
    pub fn applied_request_seq(&self) -> RequestSeq {
        RequestSeq::new(self.applied.load(Ordering::Acquire))
    }

    /// Shut down and join both worker-owned threads.
    ///
    /// A watchdog repeats cooperative cancellation while instrumented engine
    /// work joins. This cannot forcibly interrupt an uncooperative native or OS
    /// blocking call; such a call may still delay shutdown until it returns.
    pub fn shutdown(mut self) -> Result<(), WorkerFailure> {
        self.cancellation.request();
        let _ = self.commands.send(Command::Shutdown);
        let _ = self.timer_commands.send(TimerCommand::Stop);
        let worker_panicked = self.join_worker_with_watchdog();
        if worker_panicked {
            return Err(WorkerFailure::Panicked);
        }
        if self.timer.take().is_some_and(|timer| timer.join().is_err()) {
            return Err(WorkerFailure::Panicked);
        }
        Ok(())
    }
}

impl<T> Drop for LivePreviewWorker<T> {
    fn drop(&mut self) {
        self.cancellation.request();
        let _ = self.commands.send(Command::Shutdown);
        let _ = self.timer_commands.send(TimerCommand::Stop);
        self.join_worker_with_watchdog();
        if let Some(timer) = self.timer.take() {
            let _ = timer.join();
        }
    }
}

impl<T> LivePreviewWorker<T> {
    fn join_worker_with_watchdog(&mut self) -> bool {
        let Some(worker) = self.worker.take() else {
            return false;
        };
        let cancellation = self.cancellation.clone();
        let done = Arc::clone(&self.worker_done);
        let watchdog = thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                cancellation.request();
                thread::sleep(Duration::from_millis(50));
            }
        });
        let panicked = worker.join().is_err();
        self.worker_done.store(true, Ordering::Release);
        let _ = watchdog.join();
        panicked
    }
}

fn run_timer(
    commands: Receiver<TimerCommand>,
    cancellation: EngineCancellation,
    applied: Arc<AtomicU64>,
) {
    let mut pending: Option<(RequestSeq, Instant)> = None;
    loop {
        let command = match pending {
            Some((_, deadline)) => {
                commands.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            }
            None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match command {
            Ok(TimerCommand::Arm { sequence, deadline }) => pending = Some((sequence, deadline)),
            Ok(TimerCommand::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if let Some((sequence, _)) = pending.take() {
                    if applied.load(Ordering::Acquire) < sequence.get() {
                        cancellation.request();
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_worker<P: PreviewWorkerProcessor>(
    mut processor: P,
    sources: Option<SourceSet>,
    commands: Receiver<Command>,
    events: Sender<WorkerEvent<P::Output>>,
    cancellation: EngineCancellation,
    announced: Arc<AtomicU64>,
    applied_atomic: Arc<AtomicU64>,
    durable_stop: Arc<AtomicBool>,
) {
    let mut sources = match sources {
        Some(sources) => sources,
        None => loop {
            match commands.recv() {
                Ok(Command::Initialize(sources)) => break sources,
                Ok(Command::Shutdown) | Err(_) => {
                    let _ = processor.shutdown();
                    return;
                }
                Ok(_) => {
                    let _ = events.send(WorkerEvent::Fatal(
                        "preview request received before initialization".into(),
                    ));
                    let _ = processor.shutdown();
                    return;
                }
            }
        },
    };
    let mut clock = RequestClock::new();
    let mut viewport = None;
    cancellation.rearm();
    emit_compile(
        &mut processor,
        &sources,
        &events,
        RequestSeq::ZERO,
        viewport,
        &cancellation,
    );

    'worker: loop {
        let first = match commands.recv() {
            Ok(command) => command,
            Err(_) => break,
        };
        cancellation.rearm();
        let mut should_compile = false;
        let mut resync = false;
        let mut pending = Some(first);

        loop {
            let command = match pending.take().or_else(|| commands.try_recv().ok()) {
                Some(command) => command,
                None if clock.applied().get() < announced.load(Ordering::Acquire) => {
                    match commands.recv() {
                        Ok(command) => command,
                        Err(_) => break 'worker,
                    }
                }
                None => break,
            };
            match command {
                Command::Initialize(_) => {
                    let _ = events.send(WorkerEvent::Fatal(
                        "preview session was initialized more than once".into(),
                    ));
                    break 'worker;
                }
                Command::Replace {
                    sequence,
                    path,
                    text,
                } => {
                    let announced_sequence = match clock.announce() {
                        Ok(sequence) => sequence,
                        Err(error) => {
                            let _ = events.send(WorkerEvent::Fatal(error.to_string()));
                            break 'worker;
                        }
                    };
                    if announced_sequence != sequence {
                        let _ = events.send(WorkerEvent::Fatal(format!(
                            "request sequence mismatch: expected {announced_sequence}, received {sequence}"
                        )));
                        break 'worker;
                    }
                    match sources.replace(path.as_deref(), text) {
                        Ok(_) => {
                            if let Err(error) = clock.apply(sequence, ApplyDisposition::Accepted) {
                                let _ = events.send(WorkerEvent::Fatal(error.to_string()));
                                break 'worker;
                            }
                            durable_stop.store(false, Ordering::Release);
                            should_compile = true;
                        }
                        Err(error) => {
                            if let Err(clock_error) =
                                clock.apply(sequence, ApplyDisposition::Rejected)
                            {
                                let _ = events.send(WorkerEvent::Fatal(clock_error.to_string()));
                                break 'worker;
                            }
                            let _ = events.send(WorkerEvent::Rejected {
                                request_seq: sequence,
                                source_revision: sources.revision(),
                                reason: error.to_string(),
                            });
                        }
                    }
                    applied_atomic.store(sequence.get(), Ordering::Release);
                }
                Command::Viewport(first, last) => viewport = Some((first, last)),
                Command::Resync => resync = true,
                Command::Cancel => {
                    clock.stop();
                    durable_stop.store(true, Ordering::Release);
                }
                Command::Shutdown => break 'worker,
            }
        }

        if durable_stop.load(Ordering::Acquire) || clock.is_stopped() {
            let _ = events.send(WorkerEvent::Stopped {
                applied_request_seq: clock.applied(),
                source_revision: sources.revision(),
            });
            continue;
        }
        cancellation.rearm();
        if resync {
            match processor.resync(sources.snapshot(), clock.applied()) {
                Ok(output) => {
                    let _ = events.send(WorkerEvent::Update {
                        applied_request_seq: clock.applied(),
                        source_revision: sources.revision(),
                        output,
                        resync: true,
                    });
                }
                Err(error) => {
                    let _ = events.send(WorkerEvent::Aborted {
                        applied_request_seq: clock.applied(),
                        source_revision: sources.revision(),
                        reason: error.to_string(),
                    });
                }
            }
        }
        if should_compile && clock.may_compile() {
            emit_compile(
                &mut processor,
                &sources,
                &events,
                clock.applied(),
                viewport,
                &cancellation,
            );
        }
    }

    if let Err(error) = processor.shutdown() {
        let _ = events.send(WorkerEvent::Fatal(error.to_string()));
    }
}

fn emit_compile<P: PreviewWorkerProcessor>(
    processor: &mut P,
    sources: &SourceSet,
    events: &Sender<WorkerEvent<P::Output>>,
    applied_request_seq: RequestSeq,
    viewport: Option<(usize, usize)>,
    cancellation: &EngineCancellation,
) {
    let revision = sources.revision();
    match processor.compile(
        sources.snapshot(),
        applied_request_seq,
        viewport,
        cancellation,
    ) {
        WorkerRun::Complete(output) => {
            let _ = events.send(WorkerEvent::Update {
                applied_request_seq,
                source_revision: revision,
                output,
                resync: false,
            });
        }
        WorkerRun::Cancelled => {
            let _ = events.send(WorkerEvent::Cancelled {
                applied_request_seq,
                source_revision: revision,
            });
        }
        WorkerRun::Aborted(error) => {
            let _ = events.send(WorkerEvent::Aborted {
                applied_request_seq,
                source_revision: revision,
                reason: error.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use tectonic_preview_protocol::ProtocolLimits;

    use super::*;
    use crate::live_preview::DocumentRoot;

    struct MockProcessor {
        last: String,
    }

    impl PreviewWorkerProcessor for MockProcessor {
        type Output = String;
        type Error = Infallible;

        fn compile(
            &mut self,
            snapshot: AcceptedSourceSnapshot,
            _applied_request_seq: RequestSeq,
            _viewport: Option<(usize, usize)>,
            cancellation: &EngineCancellation,
        ) -> WorkerRun<Self::Output, Self::Error> {
            let text = snapshot
                .files()
                .get(snapshot.primary())
                .unwrap()
                .to_string();
            if text == "hang" {
                while !cancellation.is_requested() {
                    thread::yield_now();
                }
                return WorkerRun::Cancelled;
            }
            self.last = text.clone();
            WorkerRun::Complete(text)
        }

        fn resync(
            &mut self,
            _snapshot: AcceptedSourceSnapshot,
            _applied_request_seq: RequestSeq,
        ) -> Result<Self::Output, Self::Error> {
            Ok(self.last.clone())
        }
    }

    fn sources() -> (tempfile::TempDir, SourceSet) {
        let temp = tempfile::tempdir().unwrap();
        let root = DocumentRoot::new(temp.path()).unwrap();
        let sources = SourceSet::initialize(
            root,
            "main.tex",
            "initial".into(),
            vec![],
            ProtocolLimits::default(),
        )
        .unwrap();
        (temp, sources)
    }

    fn tuning() -> PreviewTuning {
        PreviewTuning {
            supersede_grace_ms: 10,
            ..PreviewTuning::default()
        }
    }

    #[test]
    fn hung_compile_is_superseded_and_latest_snapshot_completes() {
        let (_temp, sources) = sources();
        let mut worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
            last: String::new(),
        })
        .unwrap();
        assert!(
            matches!(worker.recv_timeout(Duration::from_secs(1)), Ok(WorkerEvent::Update { output, .. }) if output == "initial")
        );

        worker.replace(None, "hang".into()).unwrap();
        thread::sleep(Duration::from_millis(20));
        worker.replace(None, "fixed".into()).unwrap();

        let mut saw_cancel = false;
        let mut saw_fixed = false;
        for _ in 0..3 {
            match worker.recv_timeout(Duration::from_secs(1)).unwrap() {
                WorkerEvent::Cancelled { .. } => saw_cancel = true,
                WorkerEvent::Update {
                    output,
                    source_revision,
                    ..
                } if output == "fixed" => {
                    assert_eq!(source_revision, SourceRevision::new(2));
                    saw_fixed = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_cancel);
        assert!(saw_fixed);
        worker.shutdown().unwrap();
    }

    #[test]
    fn rejection_catches_up_but_cannot_revoke_user_stop() {
        let (_temp, sources) = sources();
        let mut worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
            last: String::new(),
        })
        .unwrap();
        let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.cancel().unwrap();
        worker
            .replace(Some("../escape.tex".into()), "bad".into())
            .unwrap();

        let mut rejected = false;
        let mut stopped = false;
        for _ in 0..3 {
            match worker.recv_timeout(Duration::from_secs(1)).unwrap() {
                WorkerEvent::Rejected {
                    request_seq,
                    source_revision,
                    ..
                } => {
                    assert_eq!(request_seq, RequestSeq::new(1));
                    assert_eq!(source_revision, SourceRevision::ZERO);
                    rejected = true;
                }
                WorkerEvent::Stopped { .. } => stopped = true,
                _ => {}
            }
            if rejected && stopped {
                break;
            }
        }
        assert!(rejected && stopped);
        assert_eq!(worker.applied_request_seq(), RequestSeq::new(1));

        worker.replace(None, "accepted".into()).unwrap();
        let mut accepted = false;
        for _ in 0..3 {
            if matches!(
                worker.recv_timeout(Duration::from_secs(1)),
                Ok(WorkerEvent::Update { output, source_revision, .. })
                    if output == "accepted" && source_revision == SourceRevision::new(1)
            ) {
                accepted = true;
                break;
            }
        }
        assert!(accepted);
        worker.shutdown().unwrap();
    }

    #[test]
    fn resync_reuses_last_output_without_source_revision_change() {
        let (_temp, sources) = sources();
        let worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
            last: String::new(),
        })
        .unwrap();
        let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.resync().unwrap();
        assert!(
            matches!(worker.recv_timeout(Duration::from_secs(1)), Ok(WorkerEvent::Update { output, resync: true, source_revision, .. }) if output == "initial" && source_revision == SourceRevision::ZERO)
        );
        worker.shutdown().unwrap();
    }

    #[test]
    fn bootstrap_precedes_separate_revision_zero_initialization() {
        let (_temp, sources) = sources();
        let (worker, thread_name) = LivePreviewWorker::spawn_uninitialized(tuning(), || {
            Ok::<_, Infallible>((
                MockProcessor {
                    last: String::new(),
                },
                thread::current().name().unwrap_or_default().to_owned(),
            ))
        })
        .unwrap();

        assert_eq!(thread_name, "tectonic-preview-engine");
        assert!(matches!(
            worker.recv_timeout(Duration::from_millis(10)),
            Err(RecvTimeoutError::Timeout)
        ));
        worker.initialize(sources).unwrap();
        assert!(matches!(
            worker.recv_timeout(Duration::from_secs(1)),
            Ok(WorkerEvent::Update {
                applied_request_seq: RequestSeq::ZERO,
                source_revision: SourceRevision::ZERO,
                output,
                ..
            }) if output == "initial"
        ));
        worker.shutdown().unwrap();
    }
}
