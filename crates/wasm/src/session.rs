//! One resident LaTeX project over [`tectonic_live_preview`], with the host
//! supplying files and collecting frames.
//!
//! Only one session may exist per module instance, since XeTeX keeps
//! process-wide state.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    time::SystemTime,
};

use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_engine_xdvipdfmx::XdvipdfmxEngine;
use tectonic_engine_xetex::{
    introspect::DefinedCommand, EngineCancellation, EnginePreviewConfig, TexOutcome,
};
use tectonic_errors::{
    anyhow::{anyhow, bail},
    Result,
};
use tectonic_io_base::{stack::IoStack, IoProvider, MemoryIo};
use tectonic_live_preview::{
    parse_diagnostics, AcceptedSourceSnapshot, Diagnostic, DocumentRoot, ForwardSearch,
    PreviewWorkerProcessor, PublishedFrame, ResidentPreviewError, ResidentPreviewProcessor,
    ResidentXeTeX, SourceSet, WorkerRun,
};
use tectonic_preview_protocol::{ProtocolLimits, RequestSeq, SourceFile, SourceLocation};
use tectonic_preview_render::{Destination, Link, MupdfBackend, PageVisual};

use crate::{
    host,
    io::{HostIo, HostStatus},
    resources::HostResources,
    sink::CollectingSink,
    support::{HostEngineSupport, WrittenFiles},
};

/// The virtual absolute root every source path is anchored under.
const DOCUMENT_ROOT: &str = "/doc";

// Browser tabs have far less memory than a desktop editor.
const LADDER_TARGET_BYTES: usize = 48 * 1024 * 1024;
const LADDER_HARD_BYTES: usize = 72 * 1024 * 1024;

/// What one pipeline operation returned. The sink cannot fail.
type PipelineRun =
    WorkerRun<Option<PublishedFrame>, ResidentPreviewError<std::convert::Infallible>>;

/// Present a foreground run in the same shape a tail continuation returns.
fn normalize(
    run: WorkerRun<PublishedFrame, ResidentPreviewError<std::convert::Infallible>>,
) -> PipelineRun {
    match run {
        WorkerRun::Complete(frame) => WorkerRun::Complete(Some(frame)),
        WorkerRun::Cancelled => WorkerRun::Cancelled,
        WorkerRun::Aborted(error) => WorkerRun::Aborted(error),
        WorkerRun::Fatal(error) => WorkerRun::Fatal(error),
    }
}

pub struct PageOverlay {
    pub links: Vec<Link>,
    pub destinations: Vec<Destination>,
}

pub struct RenderedPage {
    pub index: usize,
    pub svg: String,
}

/// What one compile produced. Mirrors [`crate::protocol::CompileReport`].
pub struct CompileOutput {
    pub published: bool,
    /// Only the pages whose appearance changed.
    pub pages: Vec<RenderedPage>,
    pub changed: Vec<usize>,
    pub removed: Vec<usize>,
    /// Every page, like `page_overlays`.
    pub page_sizes: Vec<[f32; 2]>,
    pub page_overlays: Vec<PageOverlay>,
    pub page_count: usize,
    pub log: Vec<u8>,
    pub outcome: &'static str,
    pub aborted: Option<String>,
    pub tail_pending: bool,
    pub messages: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub checkpoint_rungs: usize,
    pub checkpoint_bytes: usize,
    pub reused_prefix: bool,
    pub replay_rung: usize,
    pub replay_commands: u64,
    pub tex_ms: f64,
    pub analyze_ms: f64,
    pub render_ms: f64,
    pub shipouts: usize,
    pub pages_fingerprinted: usize,
    pub pages_rendered: usize,
    pub unsupported_pages: usize,
}

/// A resident project.
pub struct Session {
    processor: ResidentPreviewProcessor<HostResources, MupdfBackend, CollectingSink>,
    /// Established by the first compile.
    sources: Option<SourceSet>,
    /// Every file's text as last staged, by the caller's path spelling.
    staged: BTreeMap<String, String>,
    /// The host revision each staged file was pulled at.
    revisions: BTreeMap<String, u64>,
    written: WrittenFiles,
    /// Diagnostics accumulated across the current operation.
    messages: Rc<RefCell<Vec<String>>>,
    support: HostIo,
    cancellation: EngineCancellation,
    request_seq: RequestSeq,
    primary_name: String,
    log_name: String,
    xdv_name: String,
    pdf_name: String,
    limits: ProtocolLimits,
}

impl Session {
    /// Create a session. The format is read on the first compile.
    pub fn new(format_name: &str, primary_name: &str) -> Result<Self> {
        let cancellation = EngineCancellation::new();
        let mut config = EnginePreviewConfig::new(format_name);
        config.cancellation = cancellation.clone();
        config.ladder_target_bytes = LADDER_TARGET_BYTES;
        config.ladder_hard_bytes = LADDER_HARD_BYTES;

        let stem = primary_name.strip_suffix(".tex").unwrap_or(primary_name);
        let support = HostIo::default();
        let messages = Rc::new(RefCell::new(Vec::new()));
        let written = WrittenFiles::default();

        let engine = ResidentXeTeX::new(
            HostEngineSupport::new(support.clone(), Rc::clone(&messages), written.clone()),
            config,
        );
        let backend = MupdfBackend::new()
            .map_err(|error| anyhow!("could not start the renderer: {error}"))?;

        Ok(Self {
            processor: ResidentPreviewProcessor::new(
                engine,
                HostResources::new(support.clone()),
                backend,
                CollectingSink::default(),
            ),
            sources: None,
            staged: BTreeMap::new(),
            revisions: BTreeMap::new(),
            written,
            messages,
            support,
            cancellation,
            request_seq: RequestSeq::ZERO,
            primary_name: primary_name.to_owned(),
            log_name: format!("{stem}.log"),
            xdv_name: format!("{stem}.xdv"),
            pdf_name: format!("{stem}.pdf"),
            limits: ProtocolLimits::default(),
        })
    }

    /// Seed the support cache with bytes the host already holds.
    pub fn preload(&self, name: &str, bytes: Vec<u8>) {
        self.support.preload(name, bytes);
    }

    /// Typeset the project at its current state.
    ///
    /// `files` is the complete set of paths with their host revisions, and
    /// must include the primary. `viewport` bounds the foreground work,
    /// leaving the rest for [`continue_tail`](Self::continue_tail).
    pub fn compile(
        &mut self,
        files: BTreeMap<String, u64>,
        viewport: Option<(usize, usize)>,
    ) -> Result<CompileOutput> {
        if !files.contains_key(&self.primary_name) {
            bail!(
                "the project does not contain its primary file {}",
                self.primary_name
            );
        }

        let assembled = self.assemble(&files)?;
        let snapshot = self.apply(assembled)?;
        self.revisions = files;

        self.begin_operation()?;
        let run = self
            .processor
            .compile(snapshot, self.request_seq, viewport, &self.cancellation);
        self.collect(normalize(run))
    }

    /// Finish a viewport-bounded compile. `None` when nothing is pending.
    pub fn continue_tail(&mut self) -> Result<Option<CompileOutput>> {
        if !self.processor.has_pending_tail() {
            return Ok(None);
        }

        self.begin_operation()?;
        let run = self
            .processor
            .continue_tail(self.request_seq, &self.cancellation);

        if let WorkerRun::Complete(None) = run {
            return Ok(None);
        }
        self.collect(run).map(Some)
    }

    /// Advance the request sequence and re-arm cancellation for a new operation.
    fn begin_operation(&mut self) -> Result<()> {
        self.request_seq = self
            .request_seq
            .checked_next()
            .map_err(|_| anyhow!("request sequence exhausted"))?;
        // A cancel stops the operation it found, not every later one.
        self.cancellation.rearm();
        Ok(())
    }

    /// Ask the engine to stop at its next cooperative cancellation point.
    pub fn cancel(&self) {
        self.cancellation.request();
    }

    pub fn defined_commands(&self) -> Vec<DefinedCommand> {
        self.processor.defined_commands()
    }

    pub fn forward_search(&self, path: &str, line: usize) -> ForwardSearch {
        self.processor.forward_search(path, line)
    }

    /// Find which source line produced a point, with the path spelled
    /// relative to the document root as the caller spells it.
    pub fn backward_search(
        &self,
        page_index: usize,
        h_bp: f64,
        v_bp: f64,
    ) -> Option<SourceLocation> {
        let mut found = self.processor.backward_search(page_index, h_bp, v_bp)?;
        if let Some(relative) = found
            .path
            .strip_prefix(DOCUMENT_ROOT)
            .and_then(|path| path.strip_prefix('/'))
        {
            found.path = relative.to_owned();
        }
        Some(found)
    }

    /// Resident support files and their total bytes.
    pub fn support_stats(&self) -> (usize, usize) {
        (self.support.cached_count(), self.support.cached_bytes())
    }

    /// Run the preview's own XDV through xdvipdfmx, so the export matches
    /// what is on screen.
    ///
    /// Refuses while a tail is pending, which would truncate the PDF.
    pub fn export_pdf(&mut self) -> Result<Vec<u8>> {
        let Some(document) = self.processor.pipeline().last_document() else {
            bail!("nothing has been typeset yet");
        };

        if self.processor.has_pending_tail() {
            bail!("the document has pages left to typeset; settle the tail before exporting");
        }

        // xdvipdfmx writes to the terminal without checking the handle, so
        // the sink has to be real.
        let mut files = MemoryIo::new(true);
        files.create_entry(&self.xdv_name, document.bytes().to_vec());

        let mut support = self.support.clone();
        let mut status = HostStatus::new(Rc::clone(&self.messages));

        {
            let _registry = PdfFileRegistryLoan::take();

            let io = IoStack::new(vec![
                &mut files as &mut dyn IoProvider,
                &mut support as &mut dyn IoProvider,
            ]);
            let mut hooks = MinimalDriver::new(io);
            let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);

            let mut engine = XdvipdfmxEngine::default();
            // The default is the Unix epoch.
            engine.build_date(SystemTime::now());
            engine
                .process(&mut launcher, &self.xdv_name, &self.pdf_name)
                .map_err(|error| anyhow!("could not write the PDF: {error}"))?;
        }

        let pdf = files
            .files
            .borrow_mut()
            .remove(&self.pdf_name)
            .ok_or_else(|| anyhow!("xdvipdfmx finished without writing {}", self.pdf_name))?;
        Ok(pdf.data)
    }

    /// Release the engine's process-global resources.
    pub fn finish(&mut self) -> Result<()> {
        self.processor
            .shutdown()
            .map_err(|error| anyhow!("{error}"))
    }

    /// Resolve a revision manifest into file texts, pulling changed ones from
    /// the host.
    fn assemble(&self, files: &BTreeMap<String, u64>) -> Result<BTreeMap<String, String>> {
        let mut assembled = BTreeMap::new();
        for (path, revision) in files {
            let text = if self.revisions.get(path) == Some(revision) {
                self.staged
                    .get(path)
                    .cloned()
                    .ok_or_else(|| anyhow!("revision held for {path} but no text staged"))?
            } else {
                let bytes = host::read_source(path)
                    .map_err(|error| anyhow!("{error}"))?
                    .ok_or_else(|| anyhow!("the host has no source for {path}"))?;
                String::from_utf8(bytes)
                    .map_err(|error| anyhow!("the source {path} is not UTF-8: {error}"))?
            };
            assembled.insert(path.clone(), text);
        }
        Ok(assembled)
    }

    /// Stage this file set into the source authority, creating it on first use.
    fn apply(&mut self, mut files: BTreeMap<String, String>) -> Result<AcceptedSourceSnapshot> {
        let Some(sources) = self.sources.as_mut() else {
            let primary_text = files.remove(&self.primary_name).ok_or_else(|| {
                anyhow!(
                    "the project does not contain its primary file {}",
                    self.primary_name
                )
            })?;
            let children = files
                .iter()
                .map(|(path, text)| SourceFile {
                    path: path.clone(),
                    text: text.clone(),
                })
                .collect();

            let root = DocumentRoot::detached(DOCUMENT_ROOT)
                .map_err(|error| anyhow!("invalid document root: {error}"))?;
            let set = SourceSet::initialize(
                root,
                &self.primary_name,
                primary_text.clone(),
                children,
                self.limits,
            )
            .map_err(|error| anyhow!("invalid sources: {error}"))?;

            files.insert(self.primary_name.clone(), primary_text);
            self.staged = files;
            return Ok(self.sources.insert(set).snapshot());
        };

        // Unchanged files are skipped, since every accepted replacement
        // advances the revision even for identical bytes.
        for (path, text) in &files {
            if self.staged.get(path) == Some(text) {
                continue;
            }
            sources
                .replace(Some(path), text.clone())
                .map_err(|error| anyhow!("invalid source {path}: {error}"))?;
        }

        self.staged = files;
        Ok(sources.snapshot())
    }

    /// Turn one pipeline run into the report the host receives.
    fn collect(&mut self, run: PipelineRun) -> Result<CompileOutput> {
        let mut messages = std::mem::take(&mut *self.messages.borrow_mut());
        let log = self.written.get(&self.log_name).unwrap_or_default();
        let diagnostics = parse_diagnostics(
            &String::from_utf8_lossy(&log),
            Some(self.primary_name.as_str()),
        );

        let (outcome, aborted) = match run {
            WorkerRun::Complete(Some(published)) => (outcome_name(published.tex_outcome), None),
            WorkerRun::Complete(None) | WorkerRun::Cancelled => ("warnings", None),
            WorkerRun::Aborted(error) => ("warnings", Some(error.to_string())),
            // A fatal error poisons the session, so the host must start a new one.
            WorkerRun::Fatal(error) => bail!("the preview session failed: {error}"),
        };

        let metrics = self.processor.sink().metrics();
        let tail_pending = self.processor.sink().tail_pending();
        let frame = self.processor.sink_mut().take();
        let published = frame.is_some();

        let (pages, changed, removed, visuals) = match frame {
            Some(frame) => (
                frame
                    .pages_svg
                    .iter()
                    .map(|page| RenderedPage {
                        index: page.page_index,
                        svg: page.svg.to_string(),
                    })
                    .collect(),
                frame.changed.clone(),
                frame.removed.clone(),
                frame.pages.clone(),
            ),
            None => Default::default(),
        };

        let page_sizes: Vec<[f32; 2]> = visuals.iter().map(|page| page.size_bp).collect();
        let page_overlays = visuals
            .iter()
            .map(|page| PageOverlay {
                links: page.links.clone(),
                destinations: page.destinations.clone(),
            })
            .collect();
        let approximated = unsupported_summary(&visuals);
        if !approximated.is_empty() {
            messages.push(format!(
                "the renderer approximated: {}",
                approximated.join(", ")
            ));
        }

        Ok(CompileOutput {
            published,
            pages,
            changed,
            removed,
            page_count: page_sizes.len(),
            page_sizes,
            page_overlays,
            log,
            outcome,
            aborted,
            tail_pending,
            messages,
            diagnostics,
            checkpoint_rungs: metrics.checkpoint_rungs,
            checkpoint_bytes: metrics.checkpoint_bytes,
            reused_prefix: metrics.reused_prefix,
            replay_rung: metrics.replay_rung,
            replay_commands: metrics.replay_commands.get(),
            tex_ms: metrics.engine_ms,
            analyze_ms: metrics.analysis_ms,
            render_ms: metrics.render_ms,
            shipouts: metrics.shipouts,
            pages_fingerprinted: metrics.pages_fingerprinted,
            pages_rendered: metrics.pages_rendered,
            unsupported_pages: metrics.unsupported_pages,
        })
    }
}

/// The host API's name for an engine outcome.
fn outcome_name(outcome: TexOutcome) -> &'static str {
    match outcome {
        TexOutcome::Spotless => "spotless",
        TexOutcome::Warnings => "warnings",
        TexOutcome::Errors => "errors",
    }
}

extern "C" {
    fn pdf_files_suspend() -> *mut std::ffi::c_void;
    fn pdf_files_restore(state: *mut std::ffi::c_void);
    fn pdf_files_close_if_open();
}

/// Lends `pdf_io`'s process-global open-PDF registry to one xdvipdfmx run.
///
/// A resident XeTeX session holds it for its whole life, and `pdf_files_init`
/// asserts it is vacant. Restoring on drop survives the bridge's longjmp.
struct PdfFileRegistryLoan(*mut std::ffi::c_void);

impl PdfFileRegistryLoan {
    fn take() -> Self {
        // SAFETY: takes no arguments, called from the engines' only thread.
        Self(unsafe { pdf_files_suspend() })
    }
}

impl Drop for PdfFileRegistryLoan {
    fn drop(&mut self) {
        // SAFETY: single-threaded, and `self.0` came from pdf_files_suspend.
        // Closing first discards whatever an aborted run left installed.
        unsafe {
            pdf_files_close_if_open();
            pdf_files_restore(self.0);
        }
    }
}

/// Constructs the renderer had to approximate, counted across the document.
fn unsupported_summary(visuals: &[PageVisual]) -> Vec<String> {
    let mut counts: HashMap<&str, u64> = HashMap::new();

    for page in visuals {
        for (name, count) in &page.unsupported {
            *counts.entry(name.as_str()).or_default() += count;
        }
    }

    let mut summary: Vec<_> = counts.into_iter().collect();
    summary.sort_unstable();
    summary
        .into_iter()
        .map(|(name, count)| format!("{name} ({count})"))
        .collect()
}
