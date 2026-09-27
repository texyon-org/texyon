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
use tectonic_errors::{anyhow::bail, Result};
#[cfg(feature = "native")]
use tectonic_io_base::format_cache::FormatCache;
use tectonic_io_base::{
    filesystem::FilesystemIo, stack::IoStack, testing::SingleInputFileIo, IoProvider, MemoryIo,
};
use tectonic_status_base::NoopStatusBackend;

use super::AcceptedSourceSnapshot;
#[cfg(feature = "native")]
use super::FrozenBundle;

/// Host-specific I/O environment for resident engine operations.
///
/// Open/edit, tail continuation and teardown share one object-safe signature,
/// yielding a frame where the operation produces one.
pub trait EngineSupport {
    /// Run one engine operation against a freshly stacked I/O environment.
    ///
    /// `operation` is invoked exactly once.
    fn with_launcher(
        &mut self,
        output: &mut MemoryIo,
        operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
    ) -> Result<Option<EngineFrame>>;

    /// Tell the support layer which directory the document lives in. Defaults to a no-op.
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

fn launch(
    io: IoStack<'_>,
    operation: &mut dyn FnMut(&mut CoreBridgeLauncher<'_>) -> Result<Option<EngineFrame>>,
) -> Result<Option<EngineFrame>> {
    let mut hooks = MinimalDriver::new(io);
    let mut status = NoopStatusBackend::default();
    operation(&mut CoreBridgeLauncher::new(&mut hooks, &mut status))
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
                launch(
                    IoStack::new(vec![
                        output as &mut dyn IoProvider,
                        &mut format,
                        &mut assets,
                    ]),
                    operation,
                )
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
                let mut bundle = bundle.try_borrow_mut()?;
                providers.push(bundle.as_mut());
                providers.push(format_cache);
                launch(IoStack::new(providers), operation)
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
    // Session-lifetime, since tail continuations and fence replays re-read
    // files an earlier operation wrote (beamer's `\jobname.vrb`).
    output: MemoryIo,
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
            output: MemoryIo::new(true),
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
        let engine_snapshot = snapshot.engine_snapshot();
        let output_name = output_name(engine_snapshot.primary().as_str());
        self.output_name = Some(output_name.clone());

        let session = match self.session.take() {
            Some(session) => session,
            None => {
                self.config.cancellation = cancellation.clone();
                IncrementalXeTeXSession::new(self.config.clone())
                    .map_err(|error| ResidentEngineError::Engine(error.to_string()))?
            }
        };
        let session = self.session.insert(session);
        let page_range = viewport.map(|(first, last)| PageRange { first, last });
        let was_open = self.opened;
        // The operation is FnMut but runs once, so the snapshot moves out on that call.
        let mut pending = Some(engine_snapshot);
        let frame = self
            .support
            .with_launcher(&mut self.output, &mut |launcher| {
                let Some(sources) = pending.take() else {
                    bail!("engine operation was invoked more than once");
                };
                if was_open {
                    session.edit(launcher, sources, page_range).map(Some)
                } else {
                    session.open(launcher, sources).map(Some)
                }
            })
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))?
            .ok_or_else(|| {
                ResidentEngineError::Engine("engine operation produced no frame".into())
            })?;
        self.opened = true;

        let xdv = self.captured_xdv(&frame, &output_name)?;
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
        let frame = self
            .support
            .with_launcher(&mut self.output, &mut |launcher| {
                session.continue_tail(launcher)
            })
            .map_err(|error| ResidentEngineError::Engine(error.to_string()))?;
        let Some(frame) = frame else {
            return Ok(None);
        };
        let xdv = self.captured_xdv(&frame, &output_name)?;
        Ok(Some(ResidentEngineOutput { frame, xdv }))
    }

    fn captured_xdv(
        &self,
        frame: &EngineFrame,
        output_name: &str,
    ) -> Result<Option<Arc<[u8]>>, ResidentEngineError> {
        if frame.cancelled || frame.aborted.is_some() {
            return Ok(None);
        }
        let files = self.output.files.borrow();
        let file = files
            .get(output_name)
            .ok_or_else(|| ResidentEngineError::MissingOutput(output_name.to_owned()))?;
        Ok(Some(Arc::from(file.data.as_slice())))
    }

    /// Every control sequence the engine currently has defined, empty before the first compile.
    pub fn defined_commands(&self) -> Vec<tectonic_engine_xetex::introspect::DefinedCommand> {
        self.session
            .as_ref()
            .map(IncrementalXeTeXSession::defined_commands)
            .unwrap_or_default()
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
        self.support
            .with_launcher(&mut self.output, &mut |launcher| {
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
