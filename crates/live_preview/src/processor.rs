use std::{fmt, time::Instant};

use texyon_preview_protocol::{
    CommandCount, FrameId, FramePhase, OperationMetrics, RequestSeq, SourceLocation,
    SyncRectPayload, UpdateOutcome,
};
use texyon_preview_render::{Backend, ResourceProvider};

use texyon_engine_xetex::{EngineCancellation, EngineFrame, SyncIndex, SyncRect, TexOutcome};

use super::{
    AcceptedSourceSnapshot, DocumentFragment, FrameHeader, FrameSink, PreviewPipeline,
    PreviewPipelineError, PublicationError, ResidentEngineError, ResidentEngineOutput,
    ResidentXeTeX, SourceRevision,
};

/// A forward search answer in the last published belief.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ForwardSearch {
    /// The best box, where a viewer scrolls.
    pub result: Option<SyncRectPayload>,
    /// The boxes to flash, all on the page of `result`, best first.
    pub boxes: Vec<SyncRectPayload>,
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
    /// Unrecoverable processor or publication failure ends the session.
    Fatal(E),
}

impl<T, E> WorkerRun<T, E> {
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> WorkerRun<U, E> {
        match self {
            Self::Complete(output) => WorkerRun::Complete(f(output)),
            Self::Cancelled => WorkerRun::Cancelled,
            Self::Aborted(error) => WorkerRun::Aborted(error),
            Self::Fatal(error) => WorkerRun::Fatal(error),
        }
    }
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

    /// Whether an accepted foreground frame left resumable tail work.
    fn has_pending_tail(&self) -> bool {
        false
    }

    /// Resume deferred tail work after the foreground idle delay.
    fn continue_tail(
        &mut self,
        _applied_request_seq: RequestSeq,
        _cancellation: &EngineCancellation,
    ) -> WorkerRun<Option<Self::Output>, Self::Error> {
        WorkerRun::Complete(None)
    }

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

    /// Whether an error returned by `resync` ends the session.
    fn resync_error_is_fatal(&self, _error: &Self::Error) -> bool {
        false
    }

    /// Locate a source line in the last published belief.
    fn forward_search(&self, _path: &str, _line: usize) -> ForwardSearch {
        ForwardSearch::default()
    }

    /// Locate source text in the last successfully published belief.
    fn backward_search(
        &self,
        _page_index: usize,
        _h_bp: f64,
        _v_bp: f64,
    ) -> Option<SourceLocation> {
        None
    }
}

fn pipeline_error_is_fatal<E>(error: &PreviewPipelineError<E>) -> bool {
    matches!(
        error,
        PreviewPipelineError::Publication(PublicationError::Sink(_))
    )
}

/// Wire outcome, metrics, document fragment, and sync geometry of a completed engine frame.
fn frame_parts<'a>(
    frame: &mut EngineFrame,
    xdv: &'a [u8],
    engine_ms: f64,
) -> (
    UpdateOutcome,
    OperationMetrics,
    DocumentFragment<'a>,
    SyncIndex,
) {
    let outcome = if frame.outcome == TexOutcome::Errors {
        UpdateOutcome::Error
    } else {
        UpdateOutcome::Complete
    };
    let sync_index = std::mem::take(&mut frame.sync_index);
    let metrics = OperationMetrics {
        engine_ms,
        checkpoint_rungs: frame.checkpoint_rungs,
        checkpoint_bytes: frame.checkpoint_bytes,
        shipouts: frame.pages.len(),
        reused_prefix: frame.splice_at.is_some(),
        replay_rung: frame.replay_rung,
        replay_commands: CommandCount::new(frame.replay_commands),
        checkpoint_rungs_admitted: frame.checkpoint_rungs_admitted,
        checkpoint_rungs_dropped: frame.checkpoint_rungs_dropped,
        checkpoint_rungs_rejected: frame.checkpoint_rungs_rejected,
        ..OperationMetrics::default()
    };
    let fragment = frame
        .splice_at
        .map_or(DocumentFragment::Complete(xdv), |prefix_len| {
            DocumentFragment::Tail {
                prefix_len,
                bytes: xdv,
            }
        });
    (outcome, metrics, fragment, sync_index)
}

/// Coherent frame already accepted by the processor's publication sink.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublishedFrame {
    /// Accepted frame identity.
    pub frame_id: FrameId,
    /// Engine outcome represented on the wire.
    pub outcome: UpdateOutcome,
    /// How TeX graded the run, which tells warnings apart from a spotless run.
    pub tex_outcome: TexOutcome,
    /// Whether this foreground frame has deferred tail work.
    pub tail_pending: bool,
}

/// Engine, render, publication, or state-consistency failure.
#[derive(Debug)]
pub enum ResidentPreviewError<E> {
    /// Resident XeTeX failed terminally.
    Engine(ResidentEngineError),
    /// A non-cancelled/non-aborted engine result omitted its XDV.
    MissingXdv,
    /// Engine and coordinator source identities disagreed.
    RevisionMismatch {
        /// Coordinator-owned accepted revision.
        expected: u64,
        /// Revision reported by the engine frame.
        received: u64,
    },
    /// Assembly, rendering, or publication failed.
    Pipeline(PreviewPipelineError<E>),
    /// Recoverable TeX abort retained last-good state.
    Aborted(String),
}

impl<E: fmt::Display> fmt::Display for ResidentPreviewError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(error) => error.fmt(f),
            Self::MissingXdv => write!(f, "resident XeTeX completed without an XDV document"),
            Self::RevisionMismatch { expected, received } => write!(
                f,
                "engine source revision {received} disagrees with coordinator revision {expected}"
            ),
            Self::Pipeline(error) => error.fmt(f),
            Self::Aborted(message) => f.write_str(message),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ResidentPreviewError<E> {}

/// Complete worker-thread engine → renderer → atomic-sink processor.
pub struct ResidentPreviewProcessor<P, B, S> {
    engine: ResidentXeTeX,
    pipeline: PreviewPipeline<P, B>,
    sink: S,
    tail_revision: Option<SourceRevision>,
    /// How TeX graded the last run, for frames that do not run it again.
    tex_outcome: TexOutcome,
}

impl<P: ResourceProvider, B: Backend, S: FrameSink> ResidentPreviewProcessor<P, B, S> {
    /// Compose an unopened resident engine with renderer adapters and a frame sink.
    pub fn new(engine: ResidentXeTeX, provider: P, backend: B, sink: S) -> Self {
        Self {
            engine,
            pipeline: PreviewPipeline::new(provider, backend),
            sink,
            tail_revision: None,
            tex_outcome: TexOutcome::Spotless,
        }
    }

    /// Inspect the transactionally committed renderer/publication pipeline.
    pub fn pipeline(&self) -> &PreviewPipeline<P, B> {
        &self.pipeline
    }

    /// Reach the pipeline mutably, e.g. to draw pages on demand.
    pub fn pipeline_mut(&mut self) -> &mut PreviewPipeline<P, B> {
        &mut self.pipeline
    }

    /// Inspect the publication sink.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Every control sequence the engine currently has defined.
    pub fn defined_commands(&self) -> Vec<texyon_engine_xetex::introspect::DefinedCommand> {
        self.engine.defined_commands()
    }

    /// Reach the publication sink mutably, e.g. to drain retained frames.
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }
}

impl<P: ResourceProvider, B: Backend, S: FrameSink> ResidentPreviewProcessor<P, B, S> {
    fn publish_engine_output(
        &mut self,
        mut engine: ResidentEngineOutput,
        engine_ms: f64,
        expected_revision: SourceRevision,
        applied_request_seq: RequestSeq,
        phase: FramePhase,
        cancellation: &EngineCancellation,
    ) -> WorkerRun<PublishedFrame, ResidentPreviewError<S::Error>> {
        if engine.frame.cancelled {
            return WorkerRun::Cancelled;
        }
        if let Some(message) = engine.frame.aborted.take() {
            return WorkerRun::Aborted(ResidentPreviewError::Aborted(message));
        }
        if engine.frame.source_revision.get() != expected_revision.get() {
            return WorkerRun::Aborted(ResidentPreviewError::RevisionMismatch {
                expected: expected_revision.get(),
                received: engine.frame.source_revision.get(),
            });
        }
        let Some(xdv) = engine.xdv.take() else {
            return WorkerRun::Aborted(ResidentPreviewError::MissingXdv);
        };
        // A chunked settle leaves the rest of the tail pending too.
        let tail_pending = engine.frame.tail_pending;
        let tex_outcome = engine.frame.outcome;
        self.tex_outcome = tex_outcome;
        let (outcome, metrics, fragment, sync_index) =
            frame_parts(&mut engine.frame, &xdv, engine_ms);
        if cancellation.is_requested() {
            // Superseded after the engine committed: keep the document chain
            // coherent but skip rendering and publication.
            return match self.pipeline.absorb_fragment(fragment) {
                Ok(()) => WorkerRun::Cancelled,
                Err(error) => WorkerRun::Aborted(ResidentPreviewError::Pipeline(
                    PreviewPipelineError::Assembly(error),
                )),
            };
        }
        self.sink.set_tail_pending(tail_pending);
        let header = FrameHeader {
            source_revision: expected_revision,
            applied_request_seq,
            phase,
            outcome,
        };
        match self.pipeline.publish_fragment(
            &mut self.sink,
            fragment,
            header,
            metrics,
            tail_pending,
            sync_index,
        ) {
            Ok(frame_id) => WorkerRun::Complete(PublishedFrame {
                frame_id,
                outcome,
                tex_outcome,
                tail_pending,
            }),
            Err(error) if pipeline_error_is_fatal(&error) => {
                WorkerRun::Fatal(ResidentPreviewError::Pipeline(error))
            }
            Err(error) => WorkerRun::Aborted(ResidentPreviewError::Pipeline(error)),
        }
    }
}

impl<P, B, S> PreviewWorkerProcessor for ResidentPreviewProcessor<P, B, S>
where
    P: ResourceProvider + 'static,
    B: Backend + 'static,
    S: FrameSink + 'static,
    S::Error: fmt::Debug + fmt::Display,
{
    type Output = PublishedFrame;
    type Error = ResidentPreviewError<S::Error>;

    fn compile(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        applied_request_seq: RequestSeq,
        viewport: Option<(usize, usize)>,
        cancellation: &EngineCancellation,
    ) -> WorkerRun<Self::Output, Self::Error> {
        self.tail_revision = None;
        let revision = snapshot.revision();
        let engine_started = Instant::now();
        let engine = match self.engine.compile(snapshot, viewport, cancellation) {
            Ok(engine) => engine,
            Err(error) => return WorkerRun::Aborted(ResidentPreviewError::Engine(error)),
        };
        let run = self.publish_engine_output(
            engine,
            elapsed_ms(engine_started),
            revision,
            applied_request_seq,
            FramePhase::Foreground,
            cancellation,
        );
        if let WorkerRun::Complete(PublishedFrame {
            tail_pending: true, ..
        }) = run
        {
            self.tail_revision = Some(revision);
        }
        run
    }

    fn has_pending_tail(&self) -> bool {
        self.tail_revision.is_some() && self.engine.has_pending_tail()
    }

    fn continue_tail(
        &mut self,
        applied_request_seq: RequestSeq,
        cancellation: &EngineCancellation,
    ) -> WorkerRun<Option<Self::Output>, Self::Error> {
        let Some(expected_revision) = self.tail_revision.take() else {
            return WorkerRun::Complete(None);
        };
        let engine_started = Instant::now();
        let engine = match self.engine.continue_tail(cancellation) {
            Ok(Some(engine)) => engine,
            Ok(None) => return WorkerRun::Complete(None),
            Err(error) => return WorkerRun::Aborted(ResidentPreviewError::Engine(error)),
        };
        let run = self.publish_engine_output(
            engine,
            elapsed_ms(engine_started),
            expected_revision,
            applied_request_seq,
            FramePhase::TailSettle,
            cancellation,
        );
        if let WorkerRun::Complete(PublishedFrame {
            tail_pending: true, ..
        }) = run
        {
            self.tail_revision = Some(expected_revision);
        }
        run.map(Some)
    }

    fn resync(
        &mut self,
        _snapshot: AcceptedSourceSnapshot,
        applied_request_seq: RequestSeq,
    ) -> Result<Self::Output, Self::Error> {
        let tail_pending = self.has_pending_tail();
        self.sink.set_tail_pending(tail_pending);
        let frame_id = self
            .pipeline
            .resync(&mut self.sink, applied_request_seq)
            .map_err(ResidentPreviewError::Pipeline)?;
        Ok(PublishedFrame {
            frame_id,
            outcome: UpdateOutcome::Complete,
            tex_outcome: self.tex_outcome,
            tail_pending,
        })
    }

    fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.engine.finish().map_err(ResidentPreviewError::Engine)
    }

    fn resync_error_is_fatal(&self, error: &Self::Error) -> bool {
        matches!(error, ResidentPreviewError::Pipeline(error) if pipeline_error_is_fatal(error))
    }

    fn forward_search(&self, path: &str, line: usize) -> ForwardSearch {
        let Some(found) = self.pipeline.sync_index().highlight(path, line) else {
            return ForwardSearch::default();
        };
        let payload = |rect: &SyncRect| SyncRectPayload {
            page_index: rect.page_index,
            h_bp: sp_to_bp(rect.h),
            v_bp: sp_to_bp(rect.v),
            width_bp: sp_to_bp(rect.width),
            height_bp: sp_to_bp(rect.height),
            depth_bp: sp_to_bp(rect.depth),
        };
        ForwardSearch {
            result: Some(payload(&found.best)),
            boxes: found.boxes.iter().map(payload).collect(),
        }
    }

    fn backward_search(&self, page_index: usize, h_bp: f64, v_bp: f64) -> Option<SourceLocation> {
        self.pipeline
            .sync_index()
            .backward(page_index, bp_to_sp(h_bp), bp_to_sp(v_bp))
            .map(|rect| SourceLocation {
                path: rect.path.to_string(),
                line: rect.line,
            })
    }
}

pub(crate) fn elapsed_ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

const SP_PER_BP: f64 = 65_536.0 * 72.27 / 72.0;

fn sp_to_bp(value: i32) -> f64 {
    f64::from(value) / SP_PER_BP
}

fn bp_to_sp(value: f64) -> i32 {
    // Float-to-int `as` saturates and maps NaN to zero.
    (value * SP_PER_BP).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_publication_failure_is_fatal() {
        let closed = PreviewPipelineError::Publication(PublicationError::Sink("closed"));
        assert!(pipeline_error_is_fatal(&closed));

        let invalid = PreviewPipelineError::<&str>::Publication(
            PublicationError::InvalidRenderSet("missing page".into()),
        );
        assert!(!pipeline_error_is_fatal(&invalid));
    }
}
