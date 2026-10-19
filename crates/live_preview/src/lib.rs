//! Document-level orchestration connecting the engine and renderer for realtime preview.

mod assembler;
mod diagnostics;
mod engine_driver;
mod pipeline;
mod processor;
mod publication;
mod source;
mod transport;
mod worker_clock;

#[cfg(feature = "native")]
mod resources;
#[cfg(feature = "native")]
mod server;
#[cfg(feature = "native")]
mod worker;

#[cfg(test)]
mod test_support;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
pub use diagnostics::{parse as parse_diagnostics, Diagnostic, Severity, WRAP_WIDTH};
pub use engine_driver::{
    EngineSupport, ResidentEngineError, ResidentEngineOutput, ResidentEngineSupport, ResidentXeTeX,
};
pub use pipeline::{PreviewPipeline, PreviewPipelineError};
pub use processor::{
    ForwardSearch, PreviewWorkerProcessor, PublishedFrame, ResidentPreviewError,
    ResidentPreviewProcessor, WorkerRun,
};
pub use publication::{
    ClientBelief, FrameHeader, FramePage, FramePatch, FrameSink, PageBelief, PreviewPublisher,
    PublicationError,
};
pub use source::{AcceptedSourceSnapshot, DocumentRoot, SourceError, SourceSet};
pub use texyon_preview_protocol::{
    FrameId, FramePhase, QueryId, RequestSeq, SequenceExhausted, SourceRevision,
};
pub use transport::{JsonlFrameSink, TransportError};
pub use worker_clock::{ApplyDisposition, RequestClock, RequestClockError};

#[cfg(feature = "native")]
pub use resources::{BundleResourceProvider, FrozenBundle};
#[cfg(feature = "native")]
pub use server::{serve_preview_jsonl, PreviewBootstrap, PreviewServerError, SharedJsonlWriter};
#[cfg(feature = "native")]
pub use worker::{LivePreviewWorker, PreviewTuning, WorkerEvent, WorkerFailure};
