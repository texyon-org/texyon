use std::{fmt, time::Instant};

use tectonic_preview_protocol::{
    FrameId, FramePhase, OperationMetrics, RequestSeq, SourceLocation, SyncRectPayload,
    UpdateOutcome,
};
use tectonic_preview_render::{Backend, ResourceProvider};

use crate::engines::tex::{EngineCancellation, TexOutcome};

use super::{
    AcceptedSourceSnapshot, DocumentFragment, FrameSink, PreviewPipeline, PreviewPipelineError,
    PreviewWorkerProcessor, PublicationError, ResidentEngineError, ResidentXeTeX, SourceRevision,
    WorkerRun,
};

fn pipeline_error_is_fatal<E>(error: &PreviewPipelineError<E>) -> bool {
    matches!(
        error,
        PreviewPipelineError::Publication(PublicationError::Sink(_))
    )
}

/// Coherent frame already accepted by the processor's publication sink.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublishedFrame {
    /// Accepted frame identity.
    pub frame_id: FrameId,
    /// Engine outcome represented on the wire.
    pub outcome: UpdateOutcome,
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
}

impl<P: ResourceProvider, B: Backend, S: FrameSink> ResidentPreviewProcessor<P, B, S> {
    /// Compose an unopened resident engine with renderer adapters and a frame sink.
    pub fn new(engine: ResidentXeTeX, provider: P, backend: B, sink: S) -> Self {
        Self {
            engine,
            pipeline: PreviewPipeline::new(provider, backend),
            sink,
            tail_revision: None,
        }
    }

    /// Inspect the transactionally committed renderer/publication pipeline.
    pub fn pipeline(&self) -> &PreviewPipeline<P, B> {
        &self.pipeline
    }

    /// Inspect the publication sink.
    pub fn sink(&self) -> &S {
        &self.sink
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
        let engine_ms = engine_started.elapsed().as_secs_f64() * 1000.0;
        if engine.frame.cancelled {
            return WorkerRun::Cancelled;
        }
        if let Some(message) = engine.frame.aborted {
            return WorkerRun::Aborted(ResidentPreviewError::Aborted(message));
        }
        if engine.frame.source_revision.get() != revision.get() {
            return WorkerRun::Aborted(ResidentPreviewError::RevisionMismatch {
                expected: revision.get(),
                received: engine.frame.source_revision.get(),
            });
        }
        let Some(xdv) = engine.xdv else {
            return WorkerRun::Aborted(ResidentPreviewError::MissingXdv);
        };
        let outcome = if engine.frame.outcome == TexOutcome::Errors {
            UpdateOutcome::Error
        } else {
            UpdateOutcome::Complete
        };
        let tail_pending = engine.frame.tail_pending;
        let sync_index = engine.frame.sync_index.clone();
        let metrics = OperationMetrics {
            engine_ms,
            checkpoint_rungs: engine.frame.checkpoint_rungs,
            checkpoint_bytes: engine.frame.checkpoint_bytes,
            shipouts: engine.frame.pages.len(),
            reused_prefix: engine.frame.splice_at.is_some(),
            ..OperationMetrics::default()
        };
        self.sink.set_tail_pending(tail_pending);
        let fragment =
            engine
                .frame
                .splice_at
                .map_or(DocumentFragment::Complete(&xdv), |prefix_len| {
                    DocumentFragment::Tail {
                        prefix_len,
                        bytes: &xdv,
                    }
                });
        let frame_id = match self.pipeline.process_fragment_with_engine_ms(
            &mut self.sink,
            fragment,
            SourceRevision::new(revision.get()),
            applied_request_seq,
            FramePhase::Foreground,
            outcome,
            metrics,
            tail_pending,
            sync_index,
        ) {
            Ok(frame_id) => frame_id,
            Err(error) if pipeline_error_is_fatal(&error) => {
                return WorkerRun::Fatal(ResidentPreviewError::Pipeline(error));
            }
            Err(error) => return WorkerRun::Aborted(ResidentPreviewError::Pipeline(error)),
        };
        if tail_pending {
            self.tail_revision = Some(SourceRevision::new(revision.get()));
        }
        WorkerRun::Complete(PublishedFrame {
            frame_id,
            outcome,
            tail_pending,
        })
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
        let engine_ms = engine_started.elapsed().as_secs_f64() * 1000.0;
        if engine.frame.cancelled {
            return WorkerRun::Cancelled;
        }
        if let Some(message) = engine.frame.aborted {
            return WorkerRun::Aborted(ResidentPreviewError::Aborted(message));
        }
        if engine.frame.source_revision.get() != expected_revision.get() {
            return WorkerRun::Aborted(ResidentPreviewError::RevisionMismatch {
                expected: expected_revision.get(),
                received: engine.frame.source_revision.get(),
            });
        }
        let Some(xdv) = engine.xdv else {
            return WorkerRun::Aborted(ResidentPreviewError::MissingXdv);
        };
        let outcome = if engine.frame.outcome == TexOutcome::Errors {
            UpdateOutcome::Error
        } else {
            UpdateOutcome::Complete
        };
        let sync_index = engine.frame.sync_index.clone();
        let metrics = OperationMetrics {
            engine_ms,
            checkpoint_rungs: engine.frame.checkpoint_rungs,
            checkpoint_bytes: engine.frame.checkpoint_bytes,
            shipouts: engine.frame.pages.len(),
            reused_prefix: engine.frame.splice_at.is_some(),
            ..OperationMetrics::default()
        };
        let fragment =
            engine
                .frame
                .splice_at
                .map_or(DocumentFragment::Complete(&xdv), |prefix_len| {
                    DocumentFragment::Tail {
                        prefix_len,
                        bytes: &xdv,
                    }
                });
        self.sink.set_tail_pending(false);
        let frame_id = match self.pipeline.process_fragment_with_engine_ms(
            &mut self.sink,
            fragment,
            expected_revision,
            applied_request_seq,
            FramePhase::TailSettle,
            outcome,
            metrics,
            false,
            sync_index,
        ) {
            Ok(frame_id) => frame_id,
            Err(error) if pipeline_error_is_fatal(&error) => {
                return WorkerRun::Fatal(ResidentPreviewError::Pipeline(error));
            }
            Err(error) => return WorkerRun::Aborted(ResidentPreviewError::Pipeline(error)),
        };
        WorkerRun::Complete(Some(PublishedFrame {
            frame_id,
            outcome,
            tail_pending: false,
        }))
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
            tail_pending,
        })
    }

    fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.engine.finish().map_err(ResidentPreviewError::Engine)
    }

    fn resync_error_is_fatal(&self, error: &Self::Error) -> bool {
        matches!(error, ResidentPreviewError::Pipeline(error) if pipeline_error_is_fatal(error))
    }

    fn forward_search(&self, path: &str, line: usize) -> Option<SyncRectPayload> {
        self.pipeline
            .sync_index()
            .forward(path, line)
            .map(|rect| SyncRectPayload {
                page_index: rect.page_index,
                h_bp: sp_to_bp(rect.h),
                v_bp: sp_to_bp(rect.v),
                width_bp: sp_to_bp(rect.width),
                height_bp: sp_to_bp(rect.height),
                depth_bp: sp_to_bp(rect.depth),
            })
    }

    fn backward_search(&self, page_index: usize, h_bp: f64, v_bp: f64) -> Option<SourceLocation> {
        self.pipeline
            .sync_index()
            .backward(page_index, bp_to_sp(h_bp), bp_to_sp(v_bp))
            .map(|rect| SourceLocation {
                path: rect.path.clone(),
                line: rect.line,
            })
    }
}

const SP_PER_BP: f64 = 65_536.0 * 72.27 / 72.0;

fn sp_to_bp(value: i32) -> f64 {
    f64::from(value) / SP_PER_BP
}

fn bp_to_sp(value: f64) -> i32 {
    (value * SP_PER_BP)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
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
