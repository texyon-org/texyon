use std::convert::Infallible;

use tectonic_preview_protocol::ProtocolLimits;

use super::{run::TailCost, *};
use crate::{AcceptedSourceSnapshot, DocumentRoot, WorkerRun};

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

    fn forward_search(&self, path: &str, line: usize) -> Vec<SyncRectPayload> {
        Vec::from_iter((path == "main.tex").then_some(SyncRectPayload {
            page_index: line.saturating_sub(1),
            h_bp: 1.0,
            v_bp: 2.0,
            width_bp: 3.0,
            height_bp: 4.0,
            depth_bp: 5.0,
        }))
    }

    fn backward_search(&self, page_index: usize, _h_bp: f64, _v_bp: f64) -> Option<SourceLocation> {
        Some(SourceLocation {
            path: "main.tex".into(),
            line: page_index + 1,
        })
    }
}

struct TailProcessor {
    last: String,
    pending: bool,
}

impl PreviewWorkerProcessor for TailProcessor {
    type Output = String;
    type Error = Infallible;

    fn compile(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        _applied_request_seq: RequestSeq,
        _viewport: Option<(usize, usize)>,
        _cancellation: &EngineCancellation,
    ) -> WorkerRun<Self::Output, Self::Error> {
        self.last = snapshot
            .files()
            .get(snapshot.primary())
            .unwrap()
            .to_string();
        self.pending = self.last.starts_with("tail");
        WorkerRun::Complete(format!("foreground:{}", self.last))
    }

    fn has_pending_tail(&self) -> bool {
        self.pending
    }

    fn continue_tail(
        &mut self,
        _applied_request_seq: RequestSeq,
        cancellation: &EngineCancellation,
    ) -> WorkerRun<Option<Self::Output>, Self::Error> {
        let pending = std::mem::take(&mut self.pending);
        if pending && self.last == "tail-hang" {
            while !cancellation.is_requested() {
                thread::yield_now();
            }
            return WorkerRun::Cancelled;
        }
        WorkerRun::Complete(pending.then(|| format!("settled:{}", self.last)))
    }

    fn resync(
        &mut self,
        _snapshot: AcceptedSourceSnapshot,
        _applied_request_seq: RequestSeq,
    ) -> Result<Self::Output, Self::Error> {
        Ok(format!("resync:{}", self.last))
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
fn pending_tail_settles_after_idle_delay() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            tail_idle_ms: 10,
            ..tuning()
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "tail".into()).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, .. }) if output == "foreground:tail"
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, resync: false, .. }) if output == "settled:tail"
    ));
    worker.shutdown().unwrap();
}

#[test]
fn fresh_edit_supersedes_pending_tail_before_idle() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            tail_idle_ms: 100,
            ..tuning()
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "tail".into()).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, .. }) if output == "foreground:tail"
    ));
    worker.replace(None, "fresh".into()).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, source_revision, .. })
            if output == "foreground:fresh" && source_revision == SourceRevision::new(2)
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_millis(150)),
        Err(RecvTimeoutError::Timeout)
    ));
    worker.shutdown().unwrap();
}

#[test]
fn tail_idle_scales_with_measured_cost_and_never_shrinks_below_the_floor() {
    let floor = Duration::from_millis(200);
    let mut cost = TailCost::default();
    assert_eq!(cost.idle(floor), floor, "a cheap tail keeps the floor");

    cost.observe(Duration::from_millis(500));
    assert_eq!(
        cost.idle(floor),
        Duration::from_secs(1),
        "an expensive tail backs off proportionally"
    );

    // One cancelled continuation must not make the tail eager again.
    cost.observe(Duration::from_millis(1));
    assert!(cost.idle(floor) > Duration::from_millis(700));

    cost.observe(Duration::from_secs(30));
    assert_eq!(cost.idle(floor), TailCost::MAX_IDLE, "the delay is capped");

    assert!(
        cost.idle(Duration::ZERO).is_zero(),
        "an explicit zero delay is never stretched"
    );
}

#[test]
fn viewport_does_not_restart_tail_idle_deadline() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            tail_idle_ms: 200,
            ..tuning()
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "tail".into()).unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    thread::sleep(Duration::from_millis(120));
    worker.viewport(4, 4).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_millis(170)),
        Ok(WorkerEvent::Update { output, .. }) if output == "settled:tail"
    ));
    worker.shutdown().unwrap();
}

#[test]
fn running_tail_is_cancelled_without_foreground_grace() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            supersede_grace_ms: 2_000,
            tail_idle_ms: 0,
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "tail-hang".into()).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, .. }) if output == "foreground:tail-hang"
    ));
    thread::sleep(Duration::from_millis(20));
    let started = Instant::now();
    worker.replace(None, "fresh".into()).unwrap();
    let mut cancelled = false;
    let mut fresh = false;
    for _ in 0..3 {
        match worker.recv_timeout(Duration::from_secs(1)).unwrap() {
            WorkerEvent::Cancelled { .. } => cancelled = true,
            WorkerEvent::Update { output, .. } if output == "foreground:fresh" => {
                fresh = true;
                break;
            }
            _ => {}
        }
    }
    assert!(cancelled && fresh);
    assert!(started.elapsed() < Duration::from_secs(1));
    worker.shutdown().unwrap();
}

#[test]
fn burst_defers_every_tail_until_typing_stops() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            tail_idle_ms: 200,
            ..tuning()
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    for index in 0..10 {
        let text = format!("tail-{index}");
        worker.replace(None, text.clone()).unwrap();
        assert!(matches!(
            worker.recv_timeout(Duration::from_secs(1)),
            Ok(WorkerEvent::Update { output, .. })
                if output == format!("foreground:{text}")
        ));
    }
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, .. }) if output == "settled:tail-9"
    ));
    worker.shutdown().unwrap();
}

#[test]
fn durable_stop_drops_owed_tail() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(
        sources,
        PreviewTuning {
            tail_idle_ms: 20,
            ..tuning()
        },
        || TailProcessor {
            last: String::new(),
            pending: false,
        },
    )
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "tail".into()).unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.cancel().unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Stopped { .. })
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_millis(60)),
        Err(RecvTimeoutError::Timeout)
    ));
    worker.shutdown().unwrap();
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
fn supersession_and_shutdown_races_survive_repeated_stress() {
    for iteration in 0..20 {
        let (_temp, sources) = sources();
        let mut worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
            last: String::new(),
        })
        .unwrap();
        let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.replace(None, "hang".into()).unwrap();
        thread::sleep(Duration::from_millis(12));
        if iteration % 2 == 0 {
            worker.replace(None, format!("fixed-{iteration}")).unwrap();
            let mut completed = false;
            for _ in 0..3 {
                if matches!(
                    worker.recv_timeout(Duration::from_secs(1)),
                    Ok(WorkerEvent::Update { output, .. }) if output == format!("fixed-{iteration}")
                ) {
                    completed = true;
                    break;
                }
            }
            assert!(
                completed,
                "iteration {iteration} did not publish latest input"
            );
        }
        worker.shutdown().unwrap();
    }
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
fn navigation_queries_are_read_only_and_correlated() {
    let (_temp, sources) = sources();
    let worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
        last: String::new(),
    })
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();

    worker
        .forward_search(QueryId::new(7), "main.tex".into(), 3)
        .unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::ForwardSearchResult {
            query_id,
            boxes,
        }) if query_id == QueryId::new(7) && boxes.len() == 1 && boxes[0].page_index == 2
    ));

    worker
        .backward_search(QueryId::new(8), 4, 10.0, 20.0)
        .unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::BackwardSearchResult {
            query_id,
            result: Some(SourceLocation { path, line: 5 }),
        }) if query_id == QueryId::new(8) && path == "main.tex"
    ));
    assert_eq!(worker.applied_request_seq(), RequestSeq::ZERO);
    worker.shutdown().unwrap();
}

#[test]
fn queued_resync_runs_after_accepted_replacement_compile() {
    let (_temp, sources) = sources();
    let mut worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
        last: String::new(),
    })
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.replace(None, "edited".into()).unwrap();
    worker.resync().unwrap();

    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, resync: false, .. }) if output == "edited"
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, resync: true, .. }) if output == "edited"
    ));
    worker.shutdown().unwrap();
}

#[test]
fn durable_stop_still_allows_viewer_resync() {
    let (_temp, sources) = sources();
    let worker = LivePreviewWorker::spawn(sources, tuning(), || MockProcessor {
        last: String::new(),
    })
    .unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.cancel().unwrap();
    let _ = worker.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.resync().unwrap();

    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Update { output, resync: true, .. }) if output == "initial"
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Stopped { .. })
    ));
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

#[test]
fn fatal_processor_result_terminates_worker() {
    struct FatalProcessor;

    impl PreviewWorkerProcessor for FatalProcessor {
        type Output = ();
        type Error = &'static str;

        fn compile(
            &mut self,
            _snapshot: AcceptedSourceSnapshot,
            _applied_request_seq: RequestSeq,
            _viewport: Option<(usize, usize)>,
            _cancellation: &EngineCancellation,
        ) -> WorkerRun<Self::Output, Self::Error> {
            WorkerRun::Fatal("transport closed")
        }

        fn resync(
            &mut self,
            _snapshot: AcceptedSourceSnapshot,
            _applied_request_seq: RequestSeq,
        ) -> Result<Self::Output, Self::Error> {
            Err("transport closed")
        }
    }

    let (_temp, sources) = sources();
    let worker = LivePreviewWorker::spawn(sources, tuning(), || FatalProcessor).unwrap();
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Ok(WorkerEvent::Fatal(reason)) if reason == "transport closed"
    ));
    assert!(matches!(
        worker.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    ));
    worker.shutdown().unwrap();
}
