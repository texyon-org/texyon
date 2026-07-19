// src/engines/tex.rs -- Rustic interface to the core TeX engine.
// Copyright 2017-2021 the Tectonic Project
// Licensed under the MIT License.

//! Engine for invoking `XeTeX`.

use crate::errors::DefinitelySame;

pub use tectonic_engine_xetex::{
    EngineCancellation, EngineFrame, EnginePreviewConfig, IncrementalXeTeXSession, PageExecution,
    PageRange, SourceError, SourceIdentity, SourcePath, SourceRevision, SourceSnapshot,
    SourceUpdate, TexEngine, TexOutcome,
};

// Sigh, have to do this manually because of the Result/PartialEq conflict in errors.rs
impl DefinitelySame for TexOutcome {
    fn definitely_same(&self, other: &Self) -> bool {
        self == other
    }
}
