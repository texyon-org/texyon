use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_engine_xetex::{
    EngineCancellation, EngineFrame, EnginePreviewConfig, IncrementalXeTeXSession, PageRange,
};
use tectonic_errors::Result;
#[cfg(feature = "native")]
use tectonic_io_base::format_cache::FormatCache;
use tectonic_io_base::{
    filesystem::FilesystemIo, stack::IoStack, testing::SingleInputFileIo, IoProvider, MemoryIo,
};
use tectonic_status_base::NoopStatusBackend;

use super::AcceptedSourceSnapshot;
#[cfg(feature = "native")]
use super::FrozenBundle;

/// The I/O environment that one resident engine operation runs in.
///
/// The engine has to be handed a fresh [`CoreBridgeLauncher`] per operation,
/// and what goes underneath it differs completely by host: a bundle and a
/// format cache on a workstation, a flat fixture directory in the differential
/// tests, a blocking call back into JavaScript in a browser. This is the seam
/// between the pipeline and all of that.
///
/// Every operation is funnelled through one signature. The three the engine
/// offers — open/edit, tail continuation, teardown — differ only in whether
/// they yield a frame, so `Option<EngineFrame>` covers all three and spares
/// implementors a generic method they could not put behind a trait object.
pub trait EngineSupport {
    /// Run one engine operation against a freshly stacked I/O environment.
    ///
    /// `operation` is invoked exactly once.
    fn with_launcher(
        &mut self,
        output: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
    ) -> Result<Option<EngineFrame>>;

    /// Tell the support layer which directory the document lives in.
    ///
    /// Only meaningful where support resolution can reach the document's own
    /// files — a graphic beside `main.tex`. A host that owns every source
    /// itself has nothing to point at, so this does nothing by default.
    fn set_document_root(&mut self, _root: &Path) -> Result<()> {
        Ok(())
    }
}

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
    #[cfg(feature = "native")]
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
            #[cfg(feature = "native")]
            Self::Bundle { .. } => f.debug_struct("Bundle").finish_non_exhaustive(),
        }
    }
}

impl EngineSupport for ResidentEngineSupport {
    fn with_launcher(
        &mut self,
        output: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
    ) -> Result<Option<EngineFrame>> {
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
            #[cfg(feature = "native")]
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

    fn set_document_root(&mut self, root: &Path) -> Result<()> {
        #[cfg(feature = "native")]
        if let Self::Bundle { bundle, .. } = self {
            bundle.set_document_root(root)?;
        }
        let _ = root;
        Ok(())
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
    support: Box<dyn EngineSupport>,
    config: EnginePreviewConfig,
    session: Option<IncrementalXeTeXSession>,
    opened: bool,
    output_name: Option<String>,
}

impl ResidentXeTeX {
    /// Construct an unopened resident engine around frozen support I/O.
    pub fn new(support: impl EngineSupport + 'static, config: EnginePreviewConfig) -> Self {
        Self {
            support: Box::new(support),
            config,
            session: None,
            opened: false,
            output_name: None,
        }
    }

    /// Compile one immutable accepted snapshot, opening the resident session on first use.
    pub fn compile(
        &mut self,
        snapshot: AcceptedSourceSnapshot,
        viewport: Option<(usize, usize)>,
        cancellation: &EngineCancellation,
    ) -> Result<ResidentEngineOutput, ResidentEngineError> {
        self.support
            .set_document_root(snapshot.root())
            .map_err(|error| ResidentEngineError::Source(error.to_string()))?;
        let engine_snapshot = snapshot
            .engine_snapshot()
            .map_err(|error| ResidentEngineError::Source(error.to_string()))?;
        let output_name = output_name(engine_snapshot.primary().as_str());
        self.output_name = Some(output_name.clone());
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
        // The snapshot is consumed by the operation, but a trait object can
        // only take FnMut, so it is handed over on the single call that happens.
        let mut pending = Some(engine_snapshot);
        let frame = self
            .support
            .with_launcher(&mut output, &mut |launcher| {
                let sources = pending.take().expect("the operation runs exactly once");
                if was_open {
                    session.edit(launcher, sources, page_range).map(Some)
                } else {
                    session.open(launcher, sources).map(Some)
                }
            })
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))?
            .expect("opening and editing always produce a frame");
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

    /// Resume a pending viewport-bounded edit through the document tail.
    pub fn continue_tail(
        &mut self,
        cancellation: &EngineCancellation,
    ) -> Result<Option<ResidentEngineOutput>, ResidentEngineError> {
        let Some(session) = &mut self.session else {
            return Ok(None);
        };
        if !session.has_pending_tail() {
            return Ok(None);
        }
        self.config.cancellation = cancellation.clone();
        let output_name = self
            .output_name
            .clone()
            .ok_or_else(|| ResidentEngineError::MissingOutput("pending XDV name".into()))?;
        let mut output = MemoryIo::new(true);
        let frame = self
            .support
            .with_launcher(&mut output, &mut |launcher| session.continue_tail(launcher))
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))?;
        let Some(frame) = frame else {
            return Ok(None);
        };
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
        Ok(Some(ResidentEngineOutput { frame, xdv }))
    }

    /// Whether this resident engine retains a viewport tail continuation.
    pub fn has_pending_tail(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(IncrementalXeTeXSession::has_pending_tail)
    }

    /// Release resident native resources on the owning worker thread.
    pub fn finish(&mut self) -> Result<(), ResidentEngineError> {
        let Some(session) = &mut self.session else {
            return Ok(());
        };
        let mut output = MemoryIo::new(true);
        self.support
            .with_launcher(&mut output, &mut |launcher| {
                session.finish(launcher).map(|()| None)
            })
            .map(|_| ())
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
