use std::{
    fmt,
    sync::{
        atomic::Ordering,
        mpsc::{Receiver, RecvTimeoutError, Sender},
    },
    time::{Duration, Instant},
};

use tectonic_engine_xetex::EngineCancellation;

use super::{Command, SharedState, TimerCommand, WorkerEvent};
use crate::{
    ApplyDisposition, PreviewWorkerProcessor, RequestClock, RequestSeq, SourceRevision, SourceSet,
    WorkerRun,
};

/// Running estimate of one tail continuation's cost.
///
/// Rises at once to a larger observation and decays slowly, so one cancelled
/// continuation does not make the tail eager again.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TailCost {
    estimate: Duration,
}

impl TailCost {
    const IDLE_FACTOR: u32 = 2;
    pub(super) const MAX_IDLE: Duration = Duration::from_secs(3);

    pub(super) fn observe(&mut self, elapsed: Duration) {
        self.estimate = if elapsed > self.estimate {
            elapsed
        } else {
            (self.estimate * 3 + elapsed) / 4
        };
    }

    /// Idle delay to use, never below `configured`. A zero floor means settle immediately.
    pub(super) fn idle(&self, configured: Duration) -> Duration {
        if configured.is_zero() {
            return configured;
        }
        configured.max((self.estimate * Self::IDLE_FACTOR).min(Self::MAX_IDLE))
    }
}

pub(super) fn run_timer(
    commands: Receiver<TimerCommand>,
    cancellation: EngineCancellation,
    shared: &SharedState,
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
                    if shared.applied.load(Ordering::Acquire) < sequence.get() {
                        cancellation.request();
                    }
                }
            }
        }
    }
}

pub(super) fn run_worker<P: PreviewWorkerProcessor>(
    mut processor: P,
    sources: Option<SourceSet>,
    commands: Receiver<Command>,
    events: Sender<WorkerEvent<P::Output>>,
    cancellation: EngineCancellation,
    shared: &SharedState,
    tail_idle: Duration,
) {
    let mut sources = match sources {
        Some(sources) => sources,
        None => match commands.recv() {
            Ok(Command::Initialize(sources)) => sources,
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
        },
    };
    let mut clock = RequestClock::new();
    let mut viewport = None;
    cancellation.rearm();
    if !emit_compile(
        &mut processor,
        &sources,
        &events,
        RequestSeq::ZERO,
        viewport,
        &cancellation,
    ) {
        let _ = processor.shutdown();
        return;
    }
    // Settling the tail of a long document can cost far more than one edit,
    // so wait longer where it is expensive and settle only once typing stops.
    let mut tail_cost = TailCost::default();
    let mut tail_deadline = processor
        .has_pending_tail()
        .then(|| Instant::now() + tail_cost.idle(tail_idle));

    'worker: loop {
        let first = if processor.has_pending_tail()
            && !shared.durable_stop.load(Ordering::Acquire)
            && !clock.is_stopped()
        {
            let deadline =
                *tail_deadline.get_or_insert_with(|| Instant::now() + tail_cost.idle(tail_idle));
            match commands.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(command) => command,
                Err(RecvTimeoutError::Timeout) => {
                    cancellation.rearm();
                    tail_deadline = None;
                    shared.tail_running.store(true, Ordering::Release);
                    if shared.announced.load(Ordering::Acquire) > clock.applied().get() {
                        cancellation.request();
                    }
                    let started = Instant::now();
                    let run = processor.continue_tail(clock.applied(), &cancellation);
                    let keep_running = emit_run(&events, run, clock.applied(), sources.revision());
                    tail_cost.observe(started.elapsed());
                    shared.tail_running.store(false, Ordering::Release);
                    if !keep_running {
                        break;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match commands.recv() {
                Ok(command) => command,
                Err(_) => break,
            }
        };
        cancellation.rearm();
        let mut should_compile = false;
        let mut resync = false;
        let mut pending = Some(first);

        loop {
            let command = match pending.take().or_else(|| commands.try_recv().ok()) {
                Some(command) => command,
                None if clock.applied().get() < shared.announced.load(Ordering::Acquire) => {
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
                    match apply_replacement(
                        &mut clock,
                        &mut sources,
                        sequence,
                        path.as_deref(),
                        text,
                    ) {
                        Ok(None) => {
                            shared.durable_stop.store(false, Ordering::Release);
                            should_compile = true;
                        }
                        Ok(Some(reason)) => {
                            let _ = events.send(WorkerEvent::Rejected {
                                request_seq: sequence,
                                source_revision: sources.revision(),
                                reason,
                            });
                        }
                        Err(reason) => {
                            let _ = events.send(WorkerEvent::Fatal(reason));
                            break 'worker;
                        }
                    }
                    shared.applied.store(sequence.get(), Ordering::Release);
                }
                Command::Viewport(first, last) => viewport = Some((first, last)),
                Command::ForwardSearch {
                    query_id,
                    path,
                    line,
                } => {
                    let _ = events.send(WorkerEvent::ForwardSearchResult {
                        query_id,
                        boxes: processor.forward_search(&path, line),
                    });
                }
                Command::BackwardSearch {
                    query_id,
                    page_index,
                    h_bp,
                    v_bp,
                } => {
                    let _ = events.send(WorkerEvent::BackwardSearchResult {
                        query_id,
                        result: processor.backward_search(page_index, h_bp, v_bp),
                    });
                }
                Command::Resync => resync = true,
                Command::Cancel => {
                    clock.stop();
                    shared.durable_stop.store(true, Ordering::Release);
                    tail_deadline = None;
                }
                Command::Shutdown => break 'worker,
            }
        }

        if shared.durable_stop.load(Ordering::Acquire) || clock.is_stopped() {
            if resync && !emit_resync(&mut processor, &sources, &events, clock.applied()) {
                break 'worker;
            }
            let _ = events.send(WorkerEvent::Stopped {
                applied_request_seq: clock.applied(),
                source_revision: sources.revision(),
            });
            continue;
        }
        cancellation.rearm();
        if should_compile && clock.may_compile() {
            if !emit_compile(
                &mut processor,
                &sources,
                &events,
                clock.applied(),
                viewport,
                &cancellation,
            ) {
                break 'worker;
            }
            tail_deadline = processor
                .has_pending_tail()
                .then(|| Instant::now() + tail_cost.idle(tail_idle));
        }
        if resync && !emit_resync(&mut processor, &sources, &events, clock.applied()) {
            break 'worker;
        }
    }

    if let Err(error) = processor.shutdown() {
        let _ = events.send(WorkerEvent::Fatal(error.to_string()));
    }
}

/// Account for one announced replacement.
///
/// Returns the rejection reason if validation failed, or `Err` with a fatal
/// accounting failure.
fn apply_replacement(
    clock: &mut RequestClock,
    sources: &mut SourceSet,
    sequence: RequestSeq,
    path: Option<&str>,
    text: String,
) -> Result<Option<String>, String> {
    let announced = clock.announce().map_err(|error| error.to_string())?;
    if announced != sequence {
        return Err(format!(
            "request sequence mismatch: expected {announced}, received {sequence}"
        ));
    }
    let (disposition, rejection) = match sources.replace(path, text) {
        Ok(_) => (ApplyDisposition::Accepted, None),
        Err(error) => (ApplyDisposition::Rejected, Some(error.to_string())),
    };
    clock
        .apply(sequence, disposition)
        .map_err(|error| error.to_string())?;
    Ok(rejection)
}

fn emit_resync<P: PreviewWorkerProcessor>(
    processor: &mut P,
    sources: &SourceSet,
    events: &Sender<WorkerEvent<P::Output>>,
    applied_request_seq: RequestSeq,
) -> bool {
    match processor.resync(sources.snapshot(), applied_request_seq) {
        Ok(output) => {
            let _ = events.send(WorkerEvent::Update {
                applied_request_seq,
                source_revision: sources.revision(),
                output,
                resync: true,
            });
        }
        Err(error) if processor.resync_error_is_fatal(&error) => {
            let _ = events.send(WorkerEvent::Fatal(error.to_string()));
            return false;
        }
        Err(error) => {
            let _ = events.send(WorkerEvent::Aborted {
                applied_request_seq,
                source_revision: sources.revision(),
                reason: error.to_string(),
            });
        }
    }
    true
}

fn emit_compile<P: PreviewWorkerProcessor>(
    processor: &mut P,
    sources: &SourceSet,
    events: &Sender<WorkerEvent<P::Output>>,
    applied_request_seq: RequestSeq,
    viewport: Option<(usize, usize)>,
    cancellation: &EngineCancellation,
) -> bool {
    let run = processor
        .compile(
            sources.snapshot(),
            applied_request_seq,
            viewport,
            cancellation,
        )
        .map(Some);
    emit_run(events, run, applied_request_seq, sources.revision())
}

/// Report one compile or tail run. Returns `false` once the session must end.
fn emit_run<T, E: fmt::Display>(
    events: &Sender<WorkerEvent<T>>,
    run: WorkerRun<Option<T>, E>,
    applied_request_seq: RequestSeq,
    source_revision: SourceRevision,
) -> bool {
    let event = match run {
        WorkerRun::Complete(None) => return true,
        WorkerRun::Complete(Some(output)) => WorkerEvent::Update {
            applied_request_seq,
            source_revision,
            output,
            resync: false,
        },
        WorkerRun::Cancelled => WorkerEvent::Cancelled {
            applied_request_seq,
            source_revision,
        },
        WorkerRun::Aborted(error) => WorkerEvent::Aborted {
            applied_request_seq,
            source_revision,
            reason: error.to_string(),
        },
        WorkerRun::Fatal(error) => {
            let _ = events.send(WorkerEvent::Fatal(error.to_string()));
            return false;
        }
    };
    let _ = events.send(event);
    true
}
