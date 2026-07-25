//! One resident LaTeX project: open once, then edit per keystroke.
//!
//! This is a thin browser-shaped face over [`tectonic_live_preview`], which is
//! the same pipeline the command-line preview server runs. What it adds is a
//! host: support files that arrive by a blocking call into JavaScript, a
//! document root with no filesystem under it, and a sink that hands each frame
//! back to the caller instead of writing it to a pipe.
//!
//! Two things make an edit cheap, and they are independent. On the engine side
//! the session rewinds to the deepest checkpoint whose inputs are still
//! byte-valid rather than starting over, so the preamble, the document class
//! and every page above the cursor are not re-executed. On the rendering side
//! the pipeline fingerprints what each page *means* and draws only the pages
//! whose fingerprint moved.
//!
//! Exactly one session may exist per module instance: XeTeX keeps process-wide
//! state, which the engine enforces with a global lease.

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
    parse_diagnostics, Diagnostic, DocumentRoot, PreviewWorkerProcessor, PublishedFrame,
    ResidentPreviewError, ResidentPreviewProcessor, ResidentXeTeX, SourceSet, WorkerRun,
};
use tectonic_preview_protocol::{
    ProtocolLimits, RequestSeq, SourceFile, SourceLocation, SyncRectPayload,
};
use tectonic_preview_render::{Destination, Link, MupdfBackend, PageVisual};

use crate::{
    io::{HostIo, HostStatus},
    resources::HostResources,
    sink::CollectingSink,
    support::{HostEngineSupport, WrittenFiles},
};

/// The virtual directory the project lives in.
///
/// The engine needs an absolute root to resolve `\input` paths against, but
/// nothing here ever touches a filesystem — every source comes from the host as
/// a string and is matched by this path.
const DOCUMENT_ROOT: &str = "/doc";

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

/// What one page offers beyond its pixels.
pub struct PageOverlay {
    /// Clickable regions, in page coordinates.
    pub links: Vec<Link>,
    /// Named spots a link can reach.
    pub destinations: Vec<Destination>,
}

/// One page of the rendered document.
pub struct RenderedPage {
    /// Zero-based page index.
    pub index: usize,
    /// Self-contained SVG markup.
    pub svg: String,
}

/// What one compile produced.
pub struct CompileOutput {
    /// SVG for the pages whose appearance changed. Empty when nothing moved.
    pub pages: Vec<RenderedPage>,
    /// Indices of every page that changed, matching `pages`.
    pub changed: Vec<usize>,
    /// Pages the document lost, which the viewer should drop.
    pub removed: Vec<usize>,
    /// Size of every page in the document, in PostScript big points.
    pub page_sizes: Vec<[f32; 2]>,
    /// Clickable regions and cross-reference targets, per page.
    ///
    /// Covers the whole document rather than only the changed pages, like
    /// `page_sizes`: an overlay is cheap to carry and a viewer that had it for
    /// only some pages would be a viewer with links that sometimes work.
    pub page_overlays: Vec<PageOverlay>,
    /// Pages in the document.
    pub page_count: usize,
    /// TeX's `.log` transcript.
    pub log: Vec<u8>,
    /// Engine outcome, one of `spotless`, `warnings` or `errors`.
    pub outcome: &'static str,
    /// A recoverable fatal-class abort, if the engine hit one.
    pub aborted: Option<String>,
    /// Whether a viewport-bounded edit left a tail to finish later.
    pub tail_pending: bool,
    /// Diagnostics gathered during the run.
    pub messages: Vec<String>,
    /// What the engine said, and where, recovered from the transcript.
    pub diagnostics: Vec<Diagnostic>,
    /// Checkpoint rungs retained after the operation.
    pub checkpoint_rungs: usize,
    /// Bytes of engine state held in the checkpoint ladder.
    pub checkpoint_bytes: usize,
    /// Whether the run reused a prefix of the previous XDV.
    pub reused_prefix: bool,
    /// Checkpoint rung the run resumed from.
    ///
    /// This is what says whether the preamble was skipped: rung 0 is the start
    /// of the document, so anything above it means the class file and packages
    /// were not re-read.
    pub replay_rung: usize,
    /// TeX commands executed after the replay origin.
    pub replay_commands: u64,
    /// Milliseconds spent in XeTeX.
    pub tex_ms: f64,
    /// Milliseconds spent analyzing the XDV.
    pub analyze_ms: f64,
    /// Milliseconds spent drawing the changed pages.
    pub render_ms: f64,
    /// Fresh pages the engine shipped out.
    pub shipouts: usize,
    /// Pages the analysis fingerprinted.
    pub pages_fingerprinted: usize,
    /// Pages drawn again, of those fingerprinted.
    pub pages_rendered: usize,
    /// Pages the renderer had to approximate.
    pub unsupported_pages: usize,
}

/// A resident project.
pub struct Session {
    processor: ResidentPreviewProcessor<HostResources, MupdfBackend, CollectingSink>,
    /// Accepted source authority, once the first compile has established it.
    sources: Option<SourceSet>,
    /// Every file's text as last staged, by the caller's own path spelling.
    staged: BTreeMap<String, String>,
    /// Where operation output is published, so the `.log` outlives the run.
    written: WrittenFiles,
    /// Diagnostics accumulated across the current operation.
    messages: Rc<RefCell<Vec<String>>>,
    support: HostIo,
    cancellation: EngineCancellation,
    request_seq: RequestSeq,
    primary_name: String,
    log_name: String,
    xdv_name: String,
    limits: ProtocolLimits,
}

impl Session {
    /// Create a session against a format dump the host can supply by name.
    ///
    /// The engine is not started here: the format is read on the first
    /// [`compile`](Self::compile), under the process-global lock.
    pub fn new(format_name: &str, primary_name: &str) -> Result<Self> {
        let mut config = EnginePreviewConfig::new(format_name);
        let cancellation = EngineCancellation::new();
        config.cancellation = cancellation.clone();
        // Browser tabs are memory-constrained compared with a desktop editor,
        // and every rung is a full copy of the engine's arrays.
        config.ladder_target_bytes = 48 * 1024 * 1024;
        config.ladder_hard_bytes = 72 * 1024 * 1024;

        let stem = primary_name.strip_suffix(".tex").unwrap_or(primary_name);
        let support = HostIo::new();
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
                CollectingSink::new(),
            ),
            sources: None,
            staged: BTreeMap::new(),
            written,
            messages,
            support,
            cancellation,
            request_seq: RequestSeq::ZERO,
            primary_name: primary_name.to_owned(),
            log_name: format!("{stem}.log"),
            xdv_name: format!("{stem}.xdv"),
            limits: ProtocolLimits::default(),
        })
    }

    /// Seed the support cache with bytes the host already holds.
    ///
    /// Used for the format dump and any prefetched starter files, so the first
    /// compile does not stall on the network for them.
    pub fn preload(&self, name: &str, bytes: Vec<u8>) {
        self.support.preload(name, Rc::<[u8]>::from(bytes));
    }

    /// Typeset the project at these file contents.
    ///
    /// `files` is the project's complete current state, keyed by path relative
    /// to the document root; the primary named at construction must be among
    /// them. Files whose text is unchanged cost nothing, so handing over the
    /// whole project on every keystroke is both the intended usage and the
    /// cheap one.
    ///
    /// `viewport` bounds the foreground work to a page range, leaving the rest
    /// for [`continue_tail`](Self::continue_tail).
    pub fn compile(
        &mut self,
        files: BTreeMap<String, String>,
        viewport: Option<(usize, usize)>,
    ) -> Result<CompileOutput> {
        if !files.contains_key(&self.primary_name) {
            bail!(
                "the project does not contain its primary file {}",
                self.primary_name
            );
        }

        self.apply(files)?;

        let snapshot = self
            .sources
            .as_ref()
            .expect("apply establishes the source set")
            .snapshot();

        self.request_seq = self
            .request_seq
            .checked_next()
            .map_err(|_| anyhow!("request sequence exhausted"))?;

        let run = self
            .processor
            .compile(snapshot, self.request_seq, viewport, &self.cancellation);
        self.collect(normalize(run))
    }

    /// Finish a viewport-bounded compile through the end of the document.
    ///
    /// Returns `None` when nothing is pending.
    pub fn continue_tail(&mut self) -> Result<Option<CompileOutput>> {
        if !self.processor.has_pending_tail() {
            return Ok(None);
        }

        self.request_seq = self
            .request_seq
            .checked_next()
            .map_err(|_| anyhow!("request sequence exhausted"))?;

        let run = self
            .processor
            .continue_tail(self.request_seq, &self.cancellation);

        if let WorkerRun::Complete(None) = run {
            return Ok(None);
        }
        Ok(Some(self.collect(run)?))
    }

    /// Whether a viewport-bounded edit left work for [`continue_tail`](Self::continue_tail).
    pub fn has_pending_tail(&self) -> bool {
        self.processor.has_pending_tail()
    }

    /// Ask the engine to stop at its next cooperative cancellation point.
    pub fn cancel(&self) {
        self.cancellation.request();
    }

    /// Every command the document currently has defined.
    ///
    /// The engine's own answer, so it covers what the preamble defined, what
    /// each package defined under the options it was given, and anything built
    /// at run time that no scan of a `.sty` file could have seen.
    pub fn defined_commands(&self) -> Vec<DefinedCommand> {
        self.processor.defined_commands()
    }

    /// Find where a source line landed on the page.
    pub fn forward_search(&self, path: &str, line: usize) -> Option<SyncRectPayload> {
        self.processor.forward_search(path, line)
    }

    /// Find which source line produced a point on a page.
    ///
    /// The path comes back spelled the way the caller spells it. Internally
    /// every source is anchored under the virtual root, but a host that handed
    /// us `chapters/one.tex` should not be told about `/doc`.
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

    /// How many support files are resident, and how many bytes they hold.
    pub fn support_stats(&self) -> (usize, usize) {
        (self.support.cached_count(), self.support.cached_bytes())
    }

    /// Every support-file name the engine has asked the host for.
    pub fn requested_names(&self) -> Vec<String> {
        self.support.requested_names()
    }

    /// Convert the assembled document to PDF.
    ///
    /// This is the preview's own XDV run through xdvipdfmx, so it is the same
    /// typesetting the viewer is showing rather than a second compile — an
    /// export cannot disagree with what is on screen.
    ///
    /// Refuses while a viewport-bounded compile still has a tail: the document
    /// stops at the last page the engine was asked for, and exporting it would
    /// silently produce a truncated PDF. Settle it with
    /// [`continue_tail`](Self::continue_tail) first.
    pub fn export_pdf(&mut self) -> Result<Vec<u8>> {
        let Some(document) = self.processor.pipeline().last_document() else {
            bail!("nothing has been typeset yet");
        };

        if self.processor.has_pending_tail() {
            bail!("the document has pages left to typeset; settle the tail before exporting");
        }

        let stem = self
            .primary_name
            .strip_suffix(".tex")
            .unwrap_or(&self.primary_name);
        let pdf_name = format!("{stem}.pdf");

        // xdvipdfmx reads its input by name through the I/O stack, so the
        // assembled XDV is installed as though it had been written to disk. The
        // support layer sits underneath it, which is where the fonts it has to
        // embed come from.
        // xdvipdfmx opens the terminal to log to and does not check the
        // handle it gets, so the sink has to be real even though nothing reads
        // it back.
        let mut files = MemoryIo::new(true);
        files.create_entry(&self.xdv_name, document.bytes().to_vec());

        let mut support = self.support.clone();
        let mut status = HostStatus::new(Rc::clone(&self.messages));

        {
            // xdvipdfmx opens its own registry of the PDFs it embeds, and so
            // does XeTeX — but a *resident* XeTeX session holds that registry
            // open for its whole life, since only terminal cleanup closes it.
            // Letting xdvipdfmx initialize it a second time trips an assertion
            // inside pdf_files_init, so lend it the slot and take it back after,
            // exactly as pic_probe does when TeX itself has to measure a PDF.
            let _registry = PdfFileRegistryLoan::take();

            let io = IoStack::new(vec![
                &mut files as &mut dyn IoProvider,
                &mut support as &mut dyn IoProvider,
            ]);
            let mut hooks = MinimalDriver::new(io);
            let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);

            let mut engine = XdvipdfmxEngine::default();
            // The default is the Unix epoch, which would date every exported
            // document to 1970. The WASI shim answers the clock for real.
            engine.build_date(SystemTime::now());
            engine
                .process(&mut launcher, &self.xdv_name, &pdf_name)
                .map_err(|error| anyhow!("could not write the PDF: {error}"))?;
        }

        let pdf = files
            .files
            .borrow_mut()
            .remove(&pdf_name)
            .ok_or_else(|| anyhow!("xdvipdfmx finished without writing {pdf_name}"))?;
        Ok(pdf.data)
    }

    /// Release the engine's process-global resources.
    pub fn finish(&mut self) -> Result<()> {
        self.processor
            .shutdown()
            .map_err(|error| anyhow!("{error}"))
    }

    /// Stage this file set, establishing or updating the source authority.
    fn apply(&mut self, files: BTreeMap<String, String>) -> Result<()> {
        let Some(sources) = self.sources.as_mut() else {
            let mut files = files;
            let primary_text = files
                .remove(&self.primary_name)
                .expect("the caller checked the primary is present");
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

            self.staged = files;
            self.staged.insert(self.primary_name.clone(), primary_text);
            self.sources = Some(set);
            return Ok(());
        };

        // Only what actually changed goes to the source authority: an accepted
        // replacement advances the revision even when the bytes are identical,
        // and a project of twenty files should not look like twenty edits
        // because one of them was typed in.
        for (path, text) in &files {
            if self.staged.get(path).is_some_and(|staged| staged == text) {
                continue;
            }
            sources
                .replace(Some(path), text.clone())
                .map_err(|error| anyhow!("invalid source {path}: {error}"))?;
        }

        self.staged = files;
        Ok(())
    }

    /// Turn one pipeline run into the report the host receives.
    fn collect(&mut self, run: PipelineRun) -> Result<CompileOutput> {
        let messages = std::mem::take(&mut *self.messages.borrow_mut());
        let log = self.written.get(&self.log_name).unwrap_or_default();
        let diagnostics = parse_diagnostics(&String::from_utf8_lossy(&log));

        let mut outcome = "warnings";
        let aborted = match run {
            WorkerRun::Complete(published) => {
                if let Some(published) = published {
                    outcome = outcome_name(published.tex_outcome);
                }
                None
            }
            WorkerRun::Cancelled => None,
            WorkerRun::Aborted(error) => Some(error.to_string()),
            // A fatal error poisons the session; the host has to start a new
            // one, and saying so is the whole of what we can do about it.
            WorkerRun::Fatal(error) => bail!("the preview session failed: {error}"),
        };

        let metrics = self.processor.sink().metrics().clone();
        let tail_pending = self.processor.sink().tail_pending();
        let frame = self.processor.sink_mut().take();

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
            // Cancelled and aborted runs publish nothing, which is what keeps
            // the last good preview on screen.
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        };

        let page_sizes: Vec<[f32; 2]> = visuals.iter().map(|page| page.size_bp).collect();
        let page_overlays: Vec<PageOverlay> = visuals
            .iter()
            .map(|page| PageOverlay {
                links: page.links.clone(),
                destinations: page.destinations.clone(),
            })
            .collect();
        let mut messages = messages;
        let approximated = unsupported_summary(&visuals);
        if !approximated.is_empty() {
            messages.push(format!(
                "the renderer approximated: {}",
                approximated.join(", ")
            ));
        }

        Ok(CompileOutput {
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

/// One engine grading, as the host's API spells it.
fn outcome_name(outcome: TexOutcome) -> &'static str {
    match outcome {
        TexOutcome::Spotless => "spotless",
        TexOutcome::Warnings => "warnings",
        TexOutcome::Errors => "errors",
    }
}

/// What one pipeline operation returned.
///
/// The sink keeps frames rather than transporting them, so publication cannot
/// fail and the processor's error type collapses to the engine's own.
type PipelineRun =
    WorkerRun<Option<PublishedFrame>, ResidentPreviewError<std::convert::Infallible>>;

extern "C" {
    /// Detach the process-global open-PDF registry, yielding the old one.
    fn pdf_files_suspend() -> *mut std::ffi::c_void;
    /// Reinstall a registry taken by [`pdf_files_suspend`].
    fn pdf_files_restore(state: *mut std::ffi::c_void);
    /// Close the current registry if one is installed.
    fn pdf_files_close_if_open();
}

/// Borrows `pdf_io`'s open-PDF registry for the length of one xdvipdfmx run.
///
/// The registry is process-global, `pdf_files_init` asserts that it is vacant,
/// and a resident XeTeX session leaves its own installed from engine startup
/// until terminal cleanup — so the two cannot both hold it. Restoring on drop
/// keeps that true even when the run aborts through the bridge's longjmp.
struct PdfFileRegistryLoan(*mut std::ffi::c_void);

impl PdfFileRegistryLoan {
    /// Set the registry aside, leaving the slot vacant for a new owner.
    fn take() -> Self {
        // SAFETY: pdf_io's registry accessors take no arguments and are safe to
        // call from the single thread the engines run on.
        Self(unsafe { pdf_files_suspend() })
    }
}

impl Drop for PdfFileRegistryLoan {
    fn drop(&mut self) {
        // SAFETY: as above. A completed run has already closed its own
        // registry, making the first call a no-op; an aborted one has not, and
        // discarding what it left is what makes the slot free to give back.
        unsafe {
            pdf_files_close_if_open();
            pdf_files_restore(self.0);
        }
    }
}

/// Constructs the renderer had to approximate, gathered across the document.
fn unsupported_summary(visuals: &[PageVisual]) -> Vec<String> {
    let mut counts: HashMap<String, u64> = HashMap::new();

    for page in visuals {
        for (name, count) in &page.unsupported {
            *counts.entry(name.clone()).or_default() += count;
        }
    }

    let mut summary: Vec<_> = counts.into_iter().collect();
    summary.sort_unstable();
    summary
        .into_iter()
        .map(|(name, count)| format!("{name} ({count})"))
        .collect()
}
