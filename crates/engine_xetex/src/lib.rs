// Copyright 2021-2022 the Tectonic Project
// Licensed under the MIT License.

//! The XeTeX engine of TeXyoN, ported to Rust and kept resident for live preview.
//!
//! [`TexEngine`] runs one batch compile. [`IncrementalXeTeXSession`] keeps an
//! engine alive between edits and replays from the nearest checkpoint.

#![deny(clippy::cast_lossless)]

use std::time::SystemTime;

use texyon_bridge_core::{CoreBridgeLauncher, EngineAbortedError, engine_lock};
use texyon_errors::prelude::*;

use crate::env::EngineEnv;
use crate::state::ConfigState;

mod base;
mod build;
mod engine;
mod env;
mod ffi;
mod files;
mod fonts;
mod memory;
mod preview;
mod print;
mod scanning;
mod shipout;
mod state;
mod tables;

pub use preview::cancellation::EngineCancellation;
pub use preview::introspect;
pub use preview::page_manifest::PageExecution;
pub use preview::replay::source::{
    SourceError, SourceIdentity, SourcePath, SourceRevision, SourceSnapshot, SourceUpdate,
};
pub use preview::session::{EngineFrame, EnginePreviewConfig, IncrementalXeTeXSession, PageRange};
pub use preview::sync_index::{Highlight, SyncIndex, SyncRect};
pub use print::diagnostic::{Diagnostic, DiagnosticKind};

use crate::engine::run_engine;
use crate::preview::session::resident_active;

/// The binary layout version of format files. Formats of another serial must be regenerated.
pub const FORMAT_SERIAL: u32 = 35;

/// How a run ended. A fatal error is an `Err` instead, as are errors under `halt_on_error`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TexOutcome {
    /// Nothing bad happened.
    Spotless,

    /// Warnings were issued.
    Warnings,

    /// Errors were issued and recovered from.
    Errors,
}

/// A batch XeTeX run. Only one engine runs at a time per process, batch or resident.
#[derive(Debug)]
pub struct TexEngine {
    halt_on_error: bool,
    initex_mode: bool,
    synctex_enabled: bool,
    semantic_pagination_enabled: bool,
    shell_escape_enabled: bool,
    deterministic_mode: bool,
    build_date: SystemTime,
}

impl Default for TexEngine {
    fn default() -> Self {
        TexEngine {
            halt_on_error: true,
            initex_mode: false,
            synctex_enabled: false,
            semantic_pagination_enabled: false,
            shell_escape_enabled: false,
            deterministic_mode: false,
            build_date: SystemTime::UNIX_EPOCH,
        }
    }
}

impl TexEngine {
    /// Stop at the first error. On by default.
    pub fn halt_on_error_mode(&mut self, halt_on_error: bool) -> &mut Self {
        self.halt_on_error = halt_on_error;
        self
    }

    /// Run as INITEX and dump a format instead of typesetting.
    pub fn initex_mode(&mut self, initex: bool) -> &mut Self {
        self.initex_mode = initex;
        self
    }

    /// Write SyncTeX data.
    pub fn synctex(&mut self, synctex_enabled: bool) -> &mut Self {
        self.synctex_enabled = synctex_enabled;
        self
    }

    /// Emit SPX instead of XDV. The mode is otherwise unimplemented.
    pub fn semantic_pagination(&mut self, enabled: bool) -> &mut Self {
        self.semantic_pagination_enabled = enabled;
        self
    }

    /// Allow shell escape.
    pub fn shell_escape(&mut self, shell_escape_enabled: bool) -> &mut Self {
        self.shell_escape_enabled = shell_escape_enabled;
        self
    }

    /// Take the clock, timezone and random seed from the build date, with the elapsed timer at zero.
    pub fn deterministic_mode(&mut self, enabled: bool) -> &mut Self {
        self.deterministic_mode = enabled;
        self
    }

    /// The date `\today` and friends see. Defaults to the Unix epoch.
    pub fn build_date(&mut self, date: SystemTime) -> &mut Self {
        self.build_date = date;
        self
    }

    /// Compiles `input_file_name` (the job name, `texput` by default) with format
    /// `format_file_name`, which INITEX ignores.
    pub fn process(
        &mut self,
        launcher: &mut CoreBridgeLauncher,
        format_file_name: &str,
        input_file_name: &str,
    ) -> Result<TexOutcome> {
        if resident_active() {
            bail!("cannot start batch XeTeX while a resident XeTeX session is open");
        }
        ensure!(
            !format_file_name.contains('\0') && !input_file_name.contains('\0'),
            "XeTeX file names must not contain NUL bytes"
        );
        let config = ConfigState {
            halt_on_error_p: self.halt_on_error.into(),
            in_initex_mode: self.initex_mode,
            synctex_enabled: self.synctex_enabled.into(),
            semantic_pagination_enabled: self.semantic_pagination_enabled,
            shell_escape_enabled: self.shell_escape_enabled,
            deterministic_mode: self.deterministic_mode,
            native_pgfmath: false,
            ..ConfigState::default()
        };

        let _lock = engine_lock();
        let mut env = EngineEnv::new(launcher.host());
        let r = run_engine(
            &mut env,
            config,
            format_file_name,
            input_file_name,
            self.build_date
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("invalid build date")
                .as_secs() as libc::time_t,
        );

        match r {
            0 => Ok(TexOutcome::Spotless),
            1 => Ok(TexOutcome::Warnings),
            2 => Ok(TexOutcome::Errors),
            3 => Err(EngineAbortedError::new_with_details().into()),
            x => Err(anyhow!("internal error: unexpected 'history' value {}", x)),
        }
    }
}

mod linkage {
    // Anonymous imports deliberately make Cargo link these bridge crates.
    use {texyon_bridge_icu as _, texyon_pdf_io as _};
}

#[test]
fn linkage() {}
