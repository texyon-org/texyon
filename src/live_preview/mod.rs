//! High-level orchestration primitives for realtime preview.
//!
//! The engine and renderer deliberately remain independent. This module owns
//! the document-level state and publication rules that connect them.

mod assembler;
mod publication;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
pub use publication::{
    ClientBelief, FramePage, FramePatch, FrameSink, PageBelief, PreviewPublisher, PublicationError,
};
pub use tectonic_preview_protocol::{
    FrameId, FramePhase, RequestSeq, SequenceExhausted, SourceRevision,
};
