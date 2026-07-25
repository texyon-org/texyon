//! One resident LaTeX document: open once, then edit per keystroke.
//!
//! Two things make an edit cheap, and they are independent.
//!
//! On the engine side, [`IncrementalXeTeXSession`] journals TeX's arrays into a
//! ladder of checkpoints and, on each edit, rewinds to the deepest checkpoint
//! whose inputs are still byte-valid rather than starting over. The preamble,
//! the document class and every page above the cursor are simply not
//! re-executed — around 42,000 TeX commands instead of 1.4 million.
//!
//! On the rendering side, the same logic applies to pixels. The renderer walks
//! the XDV, fingerprints what each page *means*, and only the pages whose
//! fingerprint moved are drawn again. An edit in the last paragraph of a
//! twenty-page document redraws one page.
//!
//! Exactly one session may exist per module instance: XeTeX keeps process-wide
//! state, which the engine enforces with a global lease.

use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_engine_xetex::{
    EngineCancellation, EngineFrame, EnginePreviewConfig, IncrementalXeTeXSession, PageRange,
    SourceRevision, SourceSnapshot, TexOutcome,
};
use tectonic_errors::{
    anyhow::{anyhow, bail},
    Result,
};
use tectonic_io_base::{stack::IoStack, IoProvider};
use tectonic_preview_render::{HotRenderer, MupdfBackend, PageFingerprint, PageVisual};

use crate::{
    document::Document,
    io::{HostIo, HostStatus},
    memory_io::MemoryIo,
    resources::HostResources,
};

/// Milliseconds elapsed since `started`.
fn millis(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

/// The virtual directory the document lives in.
///
/// The engine requires an absolute root to resolve `\input` paths against, but
/// nothing here ever touches a filesystem — sources come from the host as
/// strings and are matched by this path.
const DOCUMENT_ROOT: &str = "/doc";

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
    /// Checkpoint rungs retained after the operation.
    pub checkpoint_rungs: usize,
    /// Bytes of engine state held in the checkpoint ladder.
    pub checkpoint_bytes: usize,
    /// Byte prefix of the previous XDV a fence-local replay reused.
    ///
    /// Only advances at a page boundary, so it stays zero on a one-page
    /// document even when the run was fully incremental.
    pub splice_at: Option<usize>,
    /// Checkpoint rung the run resumed from.
    ///
    /// This, not `splice_at`, is what says whether the preamble was skipped:
    /// rung 0 is the start of the document, so anything above it means the
    /// class file and packages were not re-read.
    pub replay_rung: usize,
    /// TeX commands executed after the replay origin.
    pub replay_commands: u64,
    /// Milliseconds spent in XeTeX.
    pub tex_ms: f64,
    /// Milliseconds spent analyzing the XDV.
    pub analyze_ms: f64,
    /// Milliseconds spent drawing the changed pages.
    pub render_ms: f64,
    /// Pages the analysis reused from the previous run without re-walking.
    pub reused_pages: usize,
}

/// A resident document.
pub struct Session {
    engine: IncrementalXeTeXSession,
    support: HostIo,
    renderer: HotRenderer<HostResources>,
    backend: MupdfBackend,
    /// The assembled XDV, of which an incremental run replaces only the tail.
    document: Document,
    /// Per-page visual identity as last published, for diffing.
    fingerprints: Vec<PageFingerprint>,
    cancellation: EngineCancellation,
    primary_name: String,
    xdv_name: String,
    revision: u64,
    opened: bool,
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

        Ok(Self {
            engine: IncrementalXeTeXSession::new(config)?,
            renderer: HotRenderer::new(HostResources::new(support.clone())),
            backend: MupdfBackend::new()
                .map_err(|error| anyhow!("could not start the renderer: {error}"))?,
            support,
            document: Document::default(),
            fingerprints: Vec::new(),
            cancellation,
            primary_name: primary_name.to_owned(),
            xdv_name: format!("{stem}.xdv"),
            revision: 0,
            opened: false,
        })
    }

    /// Seed the support cache with bytes the host already holds.
    ///
    /// Used for the format dump and any prefetched starter files, so the first
    /// compile does not stall on the network for them.
    pub fn preload(&self, name: &str, bytes: Vec<u8>) {
        self.support.preload(name, Rc::<[u8]>::from(bytes));
    }

    /// Replace the document's text, typeset it, and draw what changed.
    ///
    /// The first call opens the engine; later ones take the incremental path.
    /// `viewport` bounds the foreground work to a page range, leaving the rest
    /// for [`continue_tail`](Self::continue_tail).
    pub fn compile(
        &mut self,
        text: String,
        viewport: Option<(usize, usize)>,
    ) -> Result<CompileOutput> {
        let root = PathBuf::from(DOCUMENT_ROOT);
        let primary = root.join(&self.primary_name);

        let snapshot = SourceSnapshot::new_with_revision(
            root,
            primary.clone(),
            vec![(primary, Arc::<[u8]>::from(text.into_bytes()))],
            SourceRevision::new(self.revision),
        )
        .map_err(|error| anyhow!("invalid sources: {error}"))?;

        self.revision += 1;

        let page_range = viewport.map(|(first, last)| PageRange { first, last });
        let opened = self.opened;

        let started = Instant::now();
        let (frame, xdv, log, messages) = self.with_launcher(|engine, launcher| {
            if opened {
                engine.edit(launcher, snapshot, page_range)
            } else {
                engine.open(launcher, snapshot)
            }
        })?;
        let tex_ms = millis(started);

        self.opened = true;
        self.publish(frame, xdv, log, messages, tex_ms)
    }

    /// Finish a viewport-bounded compile through the end of the document.
    ///
    /// Returns `None` when nothing is pending.
    pub fn continue_tail(&mut self) -> Result<Option<CompileOutput>> {
        if !self.engine.has_pending_tail() {
            return Ok(None);
        }

        let started = Instant::now();
        let (frame, xdv, log, messages) =
            self.with_launcher(|engine, launcher| match engine.continue_tail(launcher) {
                // A settled tail reports no frame; the caller already has the
                // last one.
                Ok(None) => bail!("__tail_already_settled__"),
                Ok(Some(frame)) => Ok(frame),
                Err(error) => Err(error),
            })?;
        let tex_ms = millis(started);

        Ok(Some(self.publish(frame, xdv, log, messages, tex_ms)?))
    }

    /// Whether a viewport-bounded edit left work for [`continue_tail`](Self::continue_tail).
    pub fn has_pending_tail(&self) -> bool {
        self.engine.has_pending_tail()
    }

    /// Ask the engine to stop at its next cooperative cancellation point.
    pub fn cancel(&self) {
        self.cancellation.request();
    }

    /// How many support files are resident, and how many bytes they hold.
    pub fn support_stats(&self) -> (usize, usize) {
        (self.support.cached_count(), self.support.cached_bytes())
    }

    /// Every support-file name the engine has asked the host for.
    pub fn requested_names(&self) -> Vec<String> {
        self.support.requested_names()
    }

    /// Release the engine's process-global resources.
    pub fn finish(&mut self) -> Result<()> {
        if !self.opened {
            return Ok(());
        }

        let output = MemoryIo::new();
        let mut output_io = output.clone();
        let mut status = HostStatus::new();

        let io = IoStack::new(vec![
            &mut output_io as &mut dyn IoProvider,
            &mut self.support as &mut dyn IoProvider,
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);

        self.engine.finish(&mut launcher)?;
        self.opened = false;
        Ok(())
    }

    /// Run one engine operation against a freshly stacked I/O environment.
    fn with_launcher(
        &mut self,
        operation: impl FnOnce(
            &mut IncrementalXeTeXSession,
            &mut CoreBridgeLauncher<'_>,
        ) -> Result<EngineFrame>,
    ) -> Result<(EngineFrame, Option<Vec<u8>>, Vec<u8>, Vec<String>)> {
        let output = MemoryIo::new();
        let mut output_io = output.clone();
        let mut status = HostStatus::new();

        let frame = {
            let io = IoStack::new(vec![
                &mut output_io as &mut dyn IoProvider,
                &mut self.support as &mut dyn IoProvider,
            ]);
            let mut hooks = MinimalDriver::new(io);
            let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
            operation(&mut self.engine, &mut launcher)?
        };

        let stem = self
            .primary_name
            .strip_suffix(".tex")
            .unwrap_or(&self.primary_name);

        let xdv = output.take(&self.xdv_name);
        let log = output.get(&format!("{stem}.log")).unwrap_or_default();

        Ok((frame, xdv, log, status.take_lines()))
    }

    /// Assemble the document, work out what changed, and draw it.
    fn publish(
        &mut self,
        frame: EngineFrame,
        xdv: Option<Vec<u8>>,
        log: Vec<u8>,
        mut messages: Vec<String>,
        tex_ms: f64,
    ) -> Result<CompileOutput> {
        let mut pages = Vec::new();
        let mut changed = Vec::new();
        let mut removed = Vec::new();
        let mut page_sizes = Vec::new();
        let mut analyze_ms = 0.0;
        let mut render_ms = 0.0;
        let mut reused_pages = 0;

        // A cancelled or aborted run has no coherent XDV, and the last good
        // document stays on screen.
        if let Some(written) = xdv {
            match self.document.apply(&written, frame.splice_at) {
                Err(error) => messages.push(format!("could not assemble the document: {error}")),
                Ok(()) => {
                    let started = Instant::now();
                    let analysis = self
                        .renderer
                        .analyze_from(self.document.bytes(), self.document.page_offsets())
                        .map_err(|error| anyhow!("could not analyze the document: {error}"))?;
                    analyze_ms = millis(started);
                    reused_pages = analysis.reused_pages();

                    let visuals = analysis.pages().to_vec();
                    page_sizes = visuals.iter().map(|page| page.size_bp).collect();

                    // A page is worth redrawing exactly when what it means has
                    // moved. Pages past the end of the previous document are
                    // new, so they all count as changed.
                    changed = visuals
                        .iter()
                        .enumerate()
                        .filter(|(index, page)| {
                            self.fingerprints
                                .get(*index)
                                .is_none_or(|seen| *seen != page.fingerprint)
                        })
                        .map(|(index, _)| index)
                        .collect();

                    removed = (visuals.len()..self.fingerprints.len()).collect();

                    let started = Instant::now();
                    let rendered = self
                        .renderer
                        .render_pages(&analysis, &changed, &mut self.backend)
                        .map_err(|error| anyhow!("could not draw the document: {error}"))?;
                    render_ms = millis(started);

                    pages = rendered
                        .into_iter()
                        .map(|page| RenderedPage {
                            index: page.page_index,
                            svg: page.svg,
                        })
                        .collect();

                    // Commit the belief only once every selected page has been
                    // drawn, so a failure leaves them to be redrawn next time.
                    self.fingerprints = visuals.iter().map(|page| page.fingerprint.clone()).collect();

                    let unsupported = unsupported_summary(&visuals);
                    if !unsupported.is_empty() {
                        messages.push(format!(
                            "the renderer approximated: {}",
                            unsupported.join(", ")
                        ));
                    }
                }
            }
        }

        Ok(CompileOutput {
            pages,
            changed,
            removed,
            page_count: page_sizes.len(),
            page_sizes,
            log,
            outcome: match frame.outcome {
                TexOutcome::Spotless => "spotless",
                TexOutcome::Warnings => "warnings",
                TexOutcome::Errors => "errors",
            },
            aborted: frame.aborted,
            tail_pending: frame.tail_pending,
            messages,
            checkpoint_rungs: frame.checkpoint_rungs,
            checkpoint_bytes: frame.checkpoint_bytes,
            splice_at: frame.splice_at,
            replay_rung: frame.replay_rung,
            replay_commands: frame.replay_commands,
            tex_ms,
            analyze_ms,
            render_ms,
            reused_pages,
        })
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
