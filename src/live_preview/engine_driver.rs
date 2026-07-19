use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_io_base::filesystem::FilesystemIo;
use tectonic_status_base::NoopStatusBackend;

use crate::{
    engines::tex::{
        EngineCancellation, EngineFrame, EnginePreviewConfig, IncrementalXeTeXSession, PageRange,
    },
    io::{format_cache::FormatCache, testing::SingleInputFileIo, IoProvider, IoStack, MemoryIo},
};

use super::{AcceptedSourceSnapshot, FrozenBundle};

/// Frozen support-file universe used by one resident engine.
pub enum ResidentEngineSupport {
    /// Hermetic format plus flat support directory used by differential tests.
    Fixture {
        /// Exact format file.
        format_path: PathBuf,
        /// Read-only font/image/support directory.
        assets_dir: PathBuf,
    },
    /// Cache-only production bundle resolution chain.
    Bundle {
        /// Frozen support bundle.
        bundle: FrozenBundle,
        /// Digest-keyed prebuilt format cache.
        format_cache: FormatCache,
    },
}

impl fmt::Debug for ResidentEngineSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture {
                format_path,
                assets_dir,
            } => f
                .debug_struct("Fixture")
                .field("format_path", format_path)
                .field("assets_dir", assets_dir)
                .finish(),
            Self::Bundle { .. } => f.debug_struct("Bundle").finish_non_exhaustive(),
        }
    }
}

impl ResidentEngineSupport {
    fn with_launcher<R, E>(
        &mut self,
        output: &mut MemoryIo,
        operation: impl FnOnce(&mut CoreBridgeLauncher<'_>) -> Result<R, E>,
    ) -> Result<R, E> {
        match self {
            Self::Fixture {
                format_path,
                assets_dir,
            } => {
                let mut format = SingleInputFileIo::new(format_path);
                let mut assets = FilesystemIo::new(assets_dir, false, false, HashSet::new());
                let io = IoStack::new(vec![
                    output as &mut dyn IoProvider,
                    &mut format as &mut dyn IoProvider,
                    &mut assets as &mut dyn IoProvider,
                ]);
                let mut hooks = MinimalDriver::new(io);
                let mut status = NoopStatusBackend::default();
                let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
                operation(&mut launcher)
            }
            Self::Bundle {
                bundle,
                format_cache,
            } => {
                let document_root = bundle.document_root();
                let mut document = document_root
                    .as_ref()
                    .map(|path| FilesystemIo::new(path, false, false, HashSet::new()));
                let mut providers: Vec<&mut dyn IoProvider> = vec![output];
                if let Some(document) = &mut document {
                    providers.push(document);
                }
                let mut bundle = bundle.borrow_mut_io();
                providers.push(bundle.as_mut());
                providers.push(format_cache);
                let io = IoStack::new(providers);
                let mut hooks = MinimalDriver::new(io);
                let mut status = NoopStatusBackend::default();
                let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
                operation(&mut launcher)
            }
        }
    }
}

/// Complete result of one resident engine operation.
#[derive(Clone, Debug)]
pub struct ResidentEngineOutput {
    /// Engine warning/error, abort, cancellation, and page-execution facts.
    pub frame: EngineFrame,
    /// Coherent complete XDV, absent after cancellation or recoverable abort.
    pub xdv: Option<Arc<[u8]>>,
}

/// Source conversion, engine, or output-capture failure.
#[derive(Debug)]
pub enum ResidentEngineError {
    /// Accepted source snapshot could not enter the engine adapter.
    Source(String),
    /// Resident XeTeX operation failed terminally.
    Engine(String),
    /// Successful engine operation did not produce its expected XDV file.
    MissingOutput(String),
}

impl fmt::Display for ResidentEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "engine source snapshot is invalid: {error}"),
            Self::Engine(error) => write!(f, "resident XeTeX failed: {error}"),
            Self::MissingOutput(name) => write!(f, "resident XeTeX did not produce {name}"),
        }
    }
}

impl std::error::Error for ResidentEngineError {}

/// Worker-thread owner of a synchronous resident XeTeX session.
pub struct ResidentXeTeX {
    support: ResidentEngineSupport,
    config: EnginePreviewConfig,
    session: Option<IncrementalXeTeXSession>,
    opened: bool,
}

impl ResidentXeTeX {
    /// Construct an unopened resident engine around frozen support I/O.
    pub fn new(support: ResidentEngineSupport, config: EnginePreviewConfig) -> Self {
        Self {
            support,
            config,
            session: None,
            opened: false,
        }
    }

    /// Compile one immutable accepted snapshot, opening the resident session on first use.
    pub fn compile(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        viewport: Option<(usize, usize)>,
        cancellation: &EngineCancellation,
    ) -> Result<ResidentEngineOutput, ResidentEngineError> {
        if let ResidentEngineSupport::Bundle { bundle, .. } = &self.support {
            bundle
                .set_document_root(snapshot.root())
                .map_err(|error| ResidentEngineError::Source(error.to_string()))?;
        }
        let engine_snapshot = snapshot
            .engine_snapshot()
            .map_err(|error| ResidentEngineError::Source(error.to_string()))?;
        let output_name = output_name(engine_snapshot.primary().as_str());
        let mut output = MemoryIo::new(true);

        if self.session.is_none() {
            self.config.cancellation = cancellation.clone();
            self.session = Some(
                IncrementalXeTeXSession::new(self.config.clone())
                    .map_err(|error| ResidentEngineError::Engine(error.to_string()))?,
            );
        }
        let session = self.session.as_mut().expect("session was just initialized");
        let page_range = viewport.map(|(first, last)| PageRange { first, last });
        let was_open = self.opened;
        let frame = self
            .support
            .with_launcher(&mut output, |launcher| {
                if was_open {
                    session.edit(launcher, engine_snapshot, page_range)
                } else {
                    session.open(launcher, engine_snapshot)
                }
            })
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))?;
        self.opened = true;

        let xdv = if frame.cancelled || frame.aborted.is_some() {
            None
        } else {
            let files = output.files.borrow();
            let data = files
                .get(&output_name)
                .ok_or_else(|| ResidentEngineError::MissingOutput(output_name.clone()))?
                .data
                .clone();
            Some(Arc::from(data))
        };
        Ok(ResidentEngineOutput { frame, xdv })
    }

    /// Release resident native resources on the owning worker thread.
    pub fn finish(&mut self) -> Result<(), ResidentEngineError> {
        let Some(session) = &mut self.session else {
            return Ok(());
        };
        let mut output = MemoryIo::new(true);
        self.support
            .with_launcher(&mut output, |launcher| session.finish(launcher))
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))
    }
}

fn output_name(input: &str) -> String {
    let stem = Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(input);
    format!("{stem}.xdv")
}

#[cfg(test)]
mod tests {
    use super::output_name;

    #[test]
    fn output_name_uses_tex_job_stem() {
        assert_eq!(output_name("src/main.tex"), "main.xdv");
        assert_eq!(output_name("texput"), "texput.xdv");
    }
}
