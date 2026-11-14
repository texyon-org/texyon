// Copyright 2021-2022 the Tectonic Project
// Licensed under the MIT License.

//! The [XeTeX] program as a reusable crate.
//!
//! [XeTeX]: http://www.xetex.org/
//!
//! This crate provides the core TeX engine implementation used by [Tectonic].
//! However, in order to obtain the full Tectonic user experience, it must be
//! combined with a variety of other utilities: the `xdvipdfmx` engine, code to
//! fetch support files, and so on. Rather than using this crate directly you
//! should probably use the main [`tectonic`] crate, which combines all of these
//! pieces into a (semi) coherent whole.
//!
//! [Tectonic]: https://tectonic-typesetting.github.io/
//! [`tectonic`]: https://docs.rs/tectonic/

#![deny(clippy::cast_lossless)]

use std::time::SystemTime;

use texyon_bridge_core::{CoreBridgeLauncher, EngineAbortedError};
use texyon_errors::prelude::*;

use crate::state::ConfigState;

mod base;
mod build;
mod engine;
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

/// A serial number describing the detailed binary layout of the TeX "format
/// files" used by this crate. This number will occasionally increment,
/// indicating that the format file structure has changed. There is no provision
/// for partial forwards or backwards compatibility: if the number changes, you
/// need to regenerate your format files. If you’re generating format files, you
/// should munge this serial number in the filename, or something along those
/// lines, to make sure that when the engine is updated you don’t attempt to
/// reuse old files.
pub const FORMAT_SERIAL: u32 = 35;

/// A possible outcome from a (Xe)TeX engine invocation.
///
/// The classic TeX implementation provides a fourth outcome: "fatal error". In
/// Tectonic, this outcome is represented as an `Err` result rather than a
/// [`TexOutcome`].
///
/// The `Errors` possibility will only occur if the `halt_on_error` engine
/// option is false: if it’s true, errors get upgraded to fatals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TexOutcome {
    /// Nothing bad happened.
    Spotless,

    /// Warnings were issued by the TeX engine. Note that, due to the ways that
    /// people are used to using TeX, warnings are *extremely* common in the
    /// wild. It’s rare to find a real document that *doesn’t* compile with
    /// warnings.
    Warnings,

    /// Errors were issued by the TeX engine. Note that, in TeX terminology,
    /// errors are not necessarily *fatal* errors: the engine will try extremely
    /// hard to proceed when it encounters them. It is not uncommon to find TeX
    /// documents in the wild that produce errors.
    Errors,
}

/// A struct for invoking the (Xe)TeX engine.
///
/// This struct has a fairly straightforward "builder" interface: you create it,
/// apply any settings that you wish, and eventually run the
/// [`process()`](Self::process) method.
///
/// Due to constraints of the gnarly C/C++ code underlying the engine
/// implementation, only one engine may run at once in one process. The engine
/// execution framework uses a global mutex to ensure that this is the case.
/// This restriction applies not only to the [`TexEngine`] type but to *all*
/// Tectonic engines. I.e., you can't run this engine and the BibTeX engine at
/// the same time.
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
    /// Configure whether the engine will halt on errors.
    ///
    /// The default setting is true. If false, the engine will plunge on ahead
    /// in the face of all but the most catastrophic problems. It’s really quite
    /// impressive!
    pub fn halt_on_error_mode(&mut self, halt_on_error: bool) -> &mut Self {
        self.halt_on_error = halt_on_error;
        self
    }

    /// Configure the engine to run in "initex" mode, in which it generates a
    /// "format" file that serializes the engine state rather than a PDF
    /// document. The default is false.
    pub fn initex_mode(&mut self, initex: bool) -> &mut Self {
        self.initex_mode = initex;
        self
    }

    /// Configure the engine to produce SyncTeX data.
    ///
    /// The default is false.
    pub fn synctex(&mut self, synctex_enabled: bool) -> &mut Self {
        self.synctex_enabled = synctex_enabled;
        self
    }

    /// Configure the engine to use "semantic pagination".
    ///
    /// **Important:** this mode is essentially unimplemented.
    ///
    /// The goal of this mode is to set up the engine to create HTML-friendly
    /// output by altering how paragraphs and pages are constructed. When this
    /// mode is activated, the engine output type changes from XDV to SPX
    /// (although the two formats are quite similar).
    ///
    /// The default is false.
    pub fn semantic_pagination(&mut self, enabled: bool) -> &mut Self {
        self.semantic_pagination_enabled = enabled;
        self
    }

    /// Configure whether the "shell escape" TeX feature is enabled.
    ///
    /// The default is false.
    pub fn shell_escape(&mut self, shell_escape_enabled: bool) -> &mut Self {
        self.shell_escape_enabled = shell_escape_enabled;
        self
    }

    /// Configure deterministic engine-visible clock, timezone, and seed inputs.
    ///
    /// When enabled, calendar values and the initial random seed come from the
    /// configured build date, and the elapsed timer remains fixed at zero.
    /// The default is false, preserving XeTeX's live-clock behavior.
    pub fn deterministic_mode(&mut self, enabled: bool) -> &mut Self {
        self.deterministic_mode = enabled;
        self
    }

    /// Sets the date and time used by the TeX engine. This affects things like
    /// LaTeX's \today command.
    ///
    /// The default vaue is the Unix epoch, so you should almost always override
    /// this setting. If you are aiming to achieve reproducible builds, you will
    /// need a way to fix this parameter from one engine invocation to the next.
    pub fn build_date(&mut self, date: SystemTime) -> &mut Self {
        self.build_date = date;
        self
    }

    /// Process a document using the current engine configuration.
    ///
    /// The *launcher* parameter gives overarching environmental context in
    /// which the engine will be run.
    ///
    /// The *format_file_name* is the name for the TeX "format file" giving
    /// preloaded engine state. It must be findable in the I/O stack, using the
    /// special hooks that are provided for handing format files, which allow
    /// updates to the file format to be handed (see [`FORMAT_SERIAL`]). If in
    /// "initex" mode, this parameter will be ignored.
    ///
    /// The *input_file_name* is used to name the "primary input file". The I/O
    /// system has special hooks for opening this primary input, so be aware
    /// that this filename is *not* opened using the usual mechanisms. This
    /// setting affects some of the names used by the engine internally,
    /// including the name it uses to create its main output files. The
    /// traditional default value is `"texput"`.
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

        launcher.with_global_lock(|state| {
            let r = run_engine(
                state,
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
                3 => Err(EngineAbortedError::new_abort_indicator().into()),
                x => Err(anyhow!("internal error: unexpected 'history' value {}", x)),
            }
        })
    }
}

mod linkage {
    // Anonymous imports deliberately make Cargo link these bridge crates.
    use {texyon_bridge_icu as _, texyon_pdf_io as _};
}

#[test]
fn linkage() {}
