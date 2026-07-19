//! High-level orchestration primitives for realtime preview.
//!
//! The engine and renderer deliberately remain independent. This module owns
//! the document-level state and publication rules that connect them.

mod assembler;
mod pipeline;
mod publication;
mod source;
mod worker_clock;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
pub use pipeline::{PreviewPipeline, PreviewPipelineError};
pub use publication::{
    ClientBelief, FramePage, FramePatch, FrameSink, PageBelief, PreviewPublisher, PublicationError,
};
pub use source::{AcceptedSourceSnapshot, DocumentRoot, SourceError, SourceSet};
pub use tectonic_preview_protocol::{
    FrameId, FramePhase, RequestSeq, SequenceExhausted, SourceRevision,
};
pub use worker_clock::{ApplyDisposition, RequestClock, RequestClockError};
