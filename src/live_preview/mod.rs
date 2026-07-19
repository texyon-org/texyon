//! High-level orchestration primitives for realtime preview.
//!
//! The engine and renderer deliberately remain independent. This module owns
//! the document-level state and publication rules that connect them.

mod assembler;

pub use assembler::{
    AssembledDocument, DocumentAssembler, DocumentFragment, XdvAssemblyError, XdvFileType,
};
