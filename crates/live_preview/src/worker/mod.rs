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

use super::{PreviewWorkerProcessor, RequestSeq, SourceRevision, SourceSet};
use tectonic_engine_xetex::EngineCancellation;
use tectonic_preview_protocol::{QueryId, SourceLocation, SyncRectPayload};

use run::{run_timer, run_worker};

mod run;
#[cfg(test)]
mod tests;

const WORKER_STACK_BYTES: usize = 32 * 1024 * 1024;
const WATCHDOG_INTERVAL: Duration = Duration::from_millis(50);

/// Worker scheduling tuning fixed for one worker lifetime.
///
/// Engine-side knobs are applied to the engine configuration directly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewTuning {
    /// Delay before a newer request cancels instrumented stale work.
    pub supersede_grace_ms: u64,
    /// Idle delay before resuming a viewport-bounded document tail.
    pub tail_idle_ms: u64,
}

impl Default for PreviewTuning {
    fn default() -> Self {
        Self {
            supersede_grace_ms: 500,
            tail_idle_ms: 200,
        }
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
    /// Result of a source-to-preview query.
    ForwardSearchResult {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// The best box in the last published belief, where a viewer scrolls.
        result: Option<SyncRectPayload>,
        /// The boxes to flash, on the page of `result`, best first.
        boxes: Vec<SyncRectPayload>,
    },
    /// Result of a preview-to-source query.
    BackwardSearchResult {
        /// Client-assigned query identity.
        query_id: QueryId,
        /// Best source location in the last successfully published belief.
        result: Option<SourceLocation>,
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
    ForwardSearch {
        query_id: QueryId,
        path: String,
        line: usize,
    },
    BackwardSearch {
        query_id: QueryId,
        page_index: usize,
        h_bp: f64,
        v_bp: f64,
    },
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

/// Counters and flags read across the request, worker, and timer threads.
#[derive(Default)]
struct SharedState {
    announced: AtomicU64,
    applied: AtomicU64,
    durable_stop: AtomicBool,
    tail_running: AtomicBool,
    worker_done: AtomicBool,
}

/// Request-side handle for one resident engine worker.
pub struct LivePreviewWorker<T> {
    commands: Sender<Command>,
    events: Receiver<WorkerEvent<T>>,
    timer_commands: Sender<TimerCommand>,
    cancellation: EngineCancellation,
    shared: Arc<SharedState>,
    supersede_grace: Duration,
    next_request: RequestSeq,
    worker: Option<JoinHandle<()>>,
    timer: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> LivePreviewWorker<T> {
    /// Spawn a worker and immediately compile the validated revision-zero snapshot.
    ///
    /// `factory` runs on the worker thread, so the processor need not be `Send`.
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
    /// The bootstrap value is returned before any compile. Compilation starts
    /// once [`initialize`](Self::initialize) supplies the revision-zero snapshot.
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
        let shared = Arc::new(SharedState::default());

        let worker_cancellation = cancellation.clone();
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("tectonic-preview-engine".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || {
                match factory() {
                    Ok((processor, bootstrap)) => {
                        if bootstrap_tx.send(Ok(bootstrap)).is_ok() {
                            run_worker(
                                processor,
                                sources,
                                command_rx,
                                event_tx,
                                worker_cancellation,
                                &worker_shared,
                                Duration::from_millis(tuning.tail_idle_ms),
                            );
                        }
                    }
                    Err(error) => {
                        let _ = bootstrap_tx.send(Err(error.to_string()));
                    }
                }
                worker_shared.worker_done.store(true, Ordering::Release);
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
        let timer_shared = Arc::clone(&shared);
        let timer = thread::Builder::new()
            .name("tectonic-preview-cancel".into())
            .spawn(move || run_timer(timer_rx, timer_cancellation, &timer_shared))
            .map_err(WorkerFailure::Spawn)?;

        let worker = Self {
            commands: command_tx,
            events: event_rx,
            timer_commands: timer_tx,
            cancellation,
            shared,
            supersede_grace: Duration::from_millis(tuning.supersede_grace_ms),
            next_request: RequestSeq::ZERO,
            worker: Some(worker),
            timer: Some(timer),
        };
        Ok((worker, bootstrap))
    }

    /// Supply the one complete initialization snapshot.
    pub fn initialize(&self, sources: SourceSet) -> Result<(), WorkerFailure> {
        self.send(Command::Initialize(sources))
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
        self.shared
            .announced
            .store(sequence.get(), Ordering::Release);
        if self.shared.tail_running.load(Ordering::Acquire) {
            self.cancellation.request();
        }
        self.send(Command::Replace {
            sequence,
            path,
            text,
        })?;
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
        self.send(Command::Viewport(first, last))
    }

    /// Query preview geometry without changing request accounting or scheduling.
    pub fn forward_search(
        &self,
        query_id: QueryId,
        path: String,
        line: usize,
    ) -> Result<(), WorkerFailure> {
        self.send(Command::ForwardSearch {
            query_id,
            path,
            line,
        })
    }

    /// Query a source location without changing request accounting or scheduling.
    pub fn backward_search(
        &self,
        query_id: QueryId,
        page_index: usize,
        h_bp: f64,
        v_bp: f64,
    ) -> Result<(), WorkerFailure> {
        self.send(Command::BackwardSearch {
            query_id,
            page_index,
            h_bp,
            v_bp,
        })
    }

    /// Request a full retained-state payload without recompiling.
    pub fn resync(&self) -> Result<(), WorkerFailure> {
        self.send(Command::Resync)
    }

    /// Durably cancel current and future compilation until an accepted edit.
    pub fn cancel(&self) -> Result<(), WorkerFailure> {
        self.shared.durable_stop.store(true, Ordering::Release);
        self.cancellation.request();
        self.send(Command::Cancel)
    }

    /// Latest worker-applied request sequence.
    pub fn applied_request_seq(&self) -> RequestSeq {
        RequestSeq::new(self.shared.applied.load(Ordering::Acquire))
    }

    /// Shut down and join both worker-owned threads.
    ///
    /// Cancellation is repeated while the worker joins, but an uncooperative
    /// blocking call can still delay shutdown until it returns.
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
    fn send(&self, command: Command) -> Result<(), WorkerFailure> {
        self.commands
            .send(command)
            .map_err(|_| WorkerFailure::Closed)
    }

    fn join_worker_with_watchdog(&mut self) -> bool {
        let Some(worker) = self.worker.take() else {
            return false;
        };
        let cancellation = self.cancellation.clone();
        let shared = Arc::clone(&self.shared);
        let watchdog = thread::spawn(move || {
            while !shared.worker_done.load(Ordering::Acquire) {
                cancellation.request();
                thread::sleep(WATCHDOG_INTERVAL);
            }
        });
        let panicked = worker.join().is_err();
        self.shared.worker_done.store(true, Ordering::Release);
        let _ = watchdog.join();
        panicked
    }
}
