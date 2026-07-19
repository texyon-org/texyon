//! High-level orchestration primitives for realtime preview.
//!
//! The engine and renderer deliberately remain independent. This module owns
//! the document-level state and publication rules that connect them.

mod assembler;
mod engine_driver;
mod pipeline;
mod processor;
mod publication;
mod resources;
mod source;
mod transport;
mod worker;
mod worker_clock;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
pub use engine_driver::{
    ResidentEngineError, ResidentEngineOutput, ResidentEngineSupport, ResidentXeTeX,
};
pub use pipeline::{PreviewPipeline, PreviewPipelineError};
pub use processor::{PublishedFrame, ResidentPreviewError, ResidentPreviewProcessor};
pub use publication::{
    ClientBelief, FramePage, FramePatch, FrameSink, PageBelief, PreviewPublisher, PublicationError,
};
pub use resources::{BundleResourceProvider, FrozenBundle};
pub use source::{AcceptedSourceSnapshot, DocumentRoot, SourceError, SourceSet};
pub use tectonic_preview_protocol::{
    FrameId, FramePhase, RequestSeq, SequenceExhausted, SourceRevision,
};
pub use transport::{JsonlFrameSink, TransportError};
pub use worker::{
    LivePreviewWorker, PreviewTuning, PreviewWorkerProcessor, WorkerEvent, WorkerFailure, WorkerRun,
};
pub use worker_clock::{ApplyDisposition, RequestClock, RequestClockError};
