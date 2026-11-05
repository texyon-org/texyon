//! Document-level orchestration connecting the engine and renderer for realtime preview.

mod assembler;
mod diagnostics;
mod dispatch;
mod engine_driver;
mod pipeline;
mod processor;
mod publication;
mod raster_patch;
mod session;
mod sink;
mod source;

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
mod resources;

#[cfg(test)]
mod test_support;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
pub use diagnostics::{parse as parse_diagnostics, Diagnostic, Severity, WRAP_WIDTH};
pub use dispatch::{handle, Payload};
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
pub use session::{
    CompileOutput, PageOverlay, PreviewSession, RasterPage, RenderedPage, SessionConfig,
    SessionHost,
};
pub use sink::CollectingSink;
pub use source::{AcceptedSourceSnapshot, DocumentRoot, SourceError, SourceSet};
pub use texyon_preview_protocol::{
    FrameId, FramePhase, RequestSeq, SequenceExhausted, SourceRevision,
};

#[cfg(feature = "native")]
pub use native::{serve, DiskSources, OpenRequest, ServeError};
#[cfg(feature = "native")]
pub use resources::{BundleResourceProvider, FrozenBundle};
