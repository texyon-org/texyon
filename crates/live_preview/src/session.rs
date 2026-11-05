//! One resident LaTeX project, driven by whichever host embeds it.
//!
//! The host decides how calls arrive (a Web Worker, a native thread) and
//! supplies the engine's support files and the project's sources. Everything
//! else, from staging edits to drawing pages, lives here so every host gets
//! the same features.
//!
//! Only one session may exist per process, since XeTeX keeps process-wide
//! state.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Instant, SystemTime},
};

use texyon_engine_xdvipdfmx::XdvipdfmxEngine;
use texyon_engine_xetex::{
    introspect::DefinedCommand, EngineCancellation, EnginePreviewConfig, TexOutcome,
};
use texyon_errors::{
    anyhow::{anyhow, bail},
    Result,
};
use texyon_io_base::MemoryIo;
use texyon_preview_protocol::{
    OperationMetrics, ProtocolLimits, RequestSeq, SourceFile, SourceLocation,
};
use texyon_preview_render::{Backend, Destination, Link, PageVisual, ResourceProvider};

use crate::{
    diagnostics::{self, Diagnostic},
    AcceptedSourceSnapshot, CollectingSink, DocumentRoot, EngineSupport, ForwardSearch,
    PreviewWorkerProcessor, PublishedFrame, ResidentPreviewError, ResidentPreviewProcessor,
    ResidentXeTeX, SourceSet, WorkerRun,
};

/// Pages either side of the visible ones that the host keeps drawn.
const RASTER_OVERSCAN: usize = 1;

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

/// What the session needs from its embedder beyond the engine's support files.
pub trait SessionHost {
    /// The text of a project source the session holds no current revision
    /// of, by the caller's path spelling. `Ok(None)` means there is none.
    fn read_source(&mut self, path: &str) -> Result<Option<Vec<u8>>>;

    /// Status messages reported since the last call.
    fn take_messages(&mut self) -> Vec<String> {
        Vec::new()
    }
}

/// One splice into a file's text, in the host's UTF-16 code units.
#[derive(Clone, Debug)]
pub struct SourceEdit {
    /// The revision the offsets are into.
    pub base: u64,
    /// Start of the replaced range.
    pub from: usize,
    /// End of the replaced range, exclusive.
    pub to: usize,
    /// The replacement text.
    pub insert: String,
}

/// How a session is set up.
pub struct SessionConfig {
    /// Engine tuning. The session installs its own cancellation.
    pub engine: EnginePreviewConfig,
    /// Where source paths are anchored. A virtual root suits a host without a
    /// file system.
    pub root: DocumentRoot,
    /// The primary source, relative to `root`.
    pub primary: String,
    /// Leave pages out of compile output and draw them to pixels on request.
    pub raster: bool,
    /// Source size limits.
    pub limits: ProtocolLimits,
}

/// What a viewer draws over one page.
pub struct PageOverlay {
    /// Clickable regions.
    pub links: Vec<Link>,
    /// Named link targets.
    pub destinations: Vec<Destination>,
}

/// One page as SVG.
pub struct RenderedPage {
    /// Zero-based page index.
    pub index: usize,
    /// SVG markup.
    pub svg: String,
}

/// One page drawn to pixels, in the backend's raster layout.
pub struct RasterPage {
    /// Zero-based page index.
    pub index: usize,
    /// Width and height as little-endian `u32`, then RGBA rows.
    pub pixels: Vec<u8>,
}

/// Which pages the host shows, and at how many pixels per bp.
#[derive(Clone, Copy)]
struct RasterView {
    first: usize,
    last: usize,
    scale: f32,
}

/// What one compile produced, as a patch: `pages` and `rasters` hold only
/// changed pages.
pub struct CompileOutput {
    /// False for a cancelled or aborted run, whose page fields mean "keep
    /// what you have".
    pub published: bool,
    /// The changed pages as SVG, in a session that does not draw pixels.
    pub pages: Vec<RenderedPage>,
    /// The changed pages around the viewport, in a raster session.
    pub rasters: Vec<RasterPage>,
    /// Added or changed page indices.
    pub changed: Vec<usize>,
    /// Removed page indices.
    pub removed: Vec<usize>,
    /// Every page, like `page_overlays`.
    pub page_sizes: Vec<[f32; 2]>,
    /// Links and destinations of every page.
    pub page_overlays: Vec<PageOverlay>,
    /// Pages after this patch.
    pub page_count: usize,
    /// The TeX transcript.
    pub log: Vec<u8>,
    /// `spotless`, `warnings` or `errors`.
    pub outcome: &'static str,
    /// Why the run was abandoned, keeping the last frame.
    pub aborted: Option<String>,
    /// Whether pages past the viewport are left to typeset.
    pub tail_pending: bool,
    /// Status lines from the host and the renderer.
    pub messages: Vec<String>,
    /// Errors and warnings by source line, from the transcript.
    pub diagnostics: Vec<Diagnostic>,
    /// Engine and renderer counters, including any pages drawn to pixels.
    pub metrics: OperationMetrics,
}

/// A resident project.
pub struct PreviewSession<P, B, H> {
    processor: ResidentPreviewProcessor<P, B, CollectingSink>,
    host: H,
    root: DocumentRoot,
    /// Established by the first compile.
    sources: Option<SourceSet>,
    /// Every file's text as last staged, by the caller's path spelling.
    staged: BTreeMap<String, Staged>,
    /// The host revision each staged file was pulled at.
    revisions: BTreeMap<String, u64>,
    cancellation: EngineCancellation,
    request_seq: RequestSeq,
    primary_name: String,
    log_name: String,
    xdv_name: String,
    pdf_name: String,
    limits: ProtocolLimits,
    raster: bool,
    /// Where the last compile asked for pixels, kept for its tail.
    view: Option<RasterView>,
}

impl<P: ResourceProvider + 'static, B: Backend + 'static, H: SessionHost> PreviewSession<P, B, H> {
    /// Create a session. The format is read on the first compile.
    pub fn new(
        config: SessionConfig,
        support: impl EngineSupport + 'static,
        provider: P,
        backend: B,
        host: H,
    ) -> Self {
        let SessionConfig {
            engine: mut engine_config,
            root,
            primary,
            raster,
            limits,
        } = config;
        let cancellation = EngineCancellation::new();
        engine_config.cancellation = cancellation.clone();

        let engine = ResidentXeTeX::new(support, engine_config);
        let mut processor =
            ResidentPreviewProcessor::new(engine, provider, backend, CollectingSink::default());
        if raster {
            processor.pipeline_mut().draw_on_demand();
        }

        let stem = primary.strip_suffix(".tex").unwrap_or(&primary);
        let (log_name, xdv_name, pdf_name) = (
            format!("{stem}.log"),
            format!("{stem}.xdv"),
            format!("{stem}.pdf"),
        );

        Self {
            processor,
            host,
            root,
            sources: None,
            staged: BTreeMap::new(),
            revisions: BTreeMap::new(),
            cancellation,
            request_seq: RequestSeq::ZERO,
            primary_name: primary,
            log_name,
            xdv_name,
            pdf_name,
            limits,
            raster,
            view: None,
        }
    }

    /// The embedder's side of the session.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// A handle that stops the running operation from another thread.
    pub fn cancellation(&self) -> EngineCancellation {
        self.cancellation.clone()
    }

    /// Typeset the project at its current state.
    ///
    /// `files` is the complete set of paths with their host revisions, and
    /// must include the primary. `viewport` bounds the foreground work,
    /// leaving the rest for [`continue_tail`](Self::continue_tail).
    pub fn compile(
        &mut self,
        files: BTreeMap<String, u64>,
        edits: BTreeMap<String, SourceEdit>,
        viewport: Option<(usize, usize)>,
        scale: Option<f32>,
    ) -> Result<CompileOutput> {
        if !files.contains_key(&self.primary_name) {
            bail!(
                "the project does not contain its primary file {}",
                self.primary_name
            );
        }

        let assembled = self.assemble(&files, edits)?;
        let snapshot = self.apply(assembled)?;
        self.revisions = files;

        self.begin_operation()?;
        let run = self
            .processor
            .compile(snapshot, self.request_seq, viewport, &self.cancellation);
        self.view = viewport
            .zip(scale)
            .map(|((first, last), scale)| RasterView { first, last, scale });
        self.collect(normalize(run))
    }

    /// Draw published pages to pixels at `scale` pixels per bp.
    pub fn rasterize(&mut self, pages: &[usize], scale: f32) -> Result<Vec<RasterPage>> {
        if !self.raster {
            bail!("this session draws SVG");
        }
        let drawn = self
            .processor
            .pipeline_mut()
            .rasterize(pages, scale)
            .map_err(|error| anyhow!("could not draw the pages: {error}"))?;
        Ok(drawn
            .into_iter()
            .map(|(index, pixels)| RasterPage { index, pixels })
            .collect())
    }

    /// The changed pages the host is showing, drawn at its scale.
    fn rasterize_changed(&mut self, changed: &[usize]) -> Result<(Vec<RasterPage>, f64)> {
        let Some(view) = self.view.filter(|_| self.raster) else {
            return Ok((Vec::new(), 0.0));
        };
        let first = view.first.saturating_sub(RASTER_OVERSCAN);
        let last = view.last.saturating_add(RASTER_OVERSCAN);
        let pages: Vec<usize> = changed
            .iter()
            .copied()
            .filter(|index| (first..=last).contains(index))
            .collect();
        let started = Instant::now();
        let rasters = self.rasterize(&pages, view.scale)?;
        Ok((rasters, started.elapsed().as_secs_f64() * 1000.0))
    }

    /// Whether a viewport-bounded compile left pages to typeset.
    pub fn has_pending_tail(&self) -> bool {
        self.processor.has_pending_tail()
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

    /// Every control sequence the engine has defined, for completion.
    pub fn defined_commands(&self) -> Vec<DefinedCommand> {
        self.processor.defined_commands()
    }

    /// Find the boxes a source line produced.
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
        let root = self.root.path().to_string_lossy();
        if let Some(relative) = found
            .path
            .strip_prefix(root.as_ref())
            .and_then(|path| path.strip_prefix(['/', '\\']))
        {
            found.path = relative.to_owned();
        }
        Some(found)
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

        {
            let _registry = PdfFileRegistryLoan::take();
            let (xdv_name, pdf_name) = (self.xdv_name.as_str(), self.pdf_name.as_str());
            self.processor
                .engine_mut()
                .with_support(&mut files, &mut |launcher| {
                    let mut engine = XdvipdfmxEngine::default();
                    // The default is the Unix epoch.
                    engine.build_date(SystemTime::now());
                    engine.process(launcher, xdv_name, pdf_name)
                })
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

    /// Resolve a revision manifest into file texts, splicing in the edits
    /// made to ones held and pulling the rest from the host.
    fn assemble(
        &mut self,
        files: &BTreeMap<String, u64>,
        mut edits: BTreeMap<String, SourceEdit>,
    ) -> Result<BTreeMap<String, Staged>> {
        let mut assembled = BTreeMap::new();
        for (path, revision) in files {
            let held = self.staged.get(path);
            let held_revision = self.revisions.get(path);
            let spliced = edits.remove(path).and_then(|edit| {
                held.filter(|_| held_revision == Some(&edit.base))?
                    .splice(&edit)
            });

            let text = if held_revision == Some(revision) {
                held.cloned()
                    .ok_or_else(|| anyhow!("revision held for {path} but no text staged"))?
            } else if let Some(spliced) = spliced {
                spliced
            } else {
                let bytes = self
                    .host
                    .read_source(path)?
                    .ok_or_else(|| anyhow!("the host has no source for {path}"))?;
                Staged::new(
                    String::from_utf8(bytes)
                        .map_err(|error| anyhow!("the source {path} is not UTF-8: {error}"))?,
                )
            };
            assembled.insert(path.clone(), text);
        }
        Ok(assembled)
    }

    /// Stage this file set into the source authority, creating it on first use.
    fn apply(&mut self, files: BTreeMap<String, Staged>) -> Result<AcceptedSourceSnapshot> {
        let Some(sources) = self.sources.as_mut() else {
            let primary_text = files.get(&self.primary_name).ok_or_else(|| {
                anyhow!(
                    "the project does not contain its primary file {}",
                    self.primary_name
                )
            })?;
            let children = files
                .iter()
                .filter(|(path, _)| **path != self.primary_name)
                .map(|(path, staged)| SourceFile {
                    path: path.clone(),
                    text: staged.text.to_string(),
                })
                .collect();

            let set = SourceSet::initialize(
                self.root.clone(),
                &self.primary_name,
                primary_text.text.to_string(),
                children,
                self.limits,
            )
            .map_err(|error| anyhow!("invalid sources: {error}"))?;

            self.staged = files;
            return Ok(self.sources.insert(set).snapshot());
        };

        // Unchanged files are skipped, since every accepted replacement
        // advances the revision even for identical bytes.
        for (path, staged) in &files {
            if self.staged.get(path).map(|held| &held.text) == Some(&staged.text) {
                continue;
            }
            sources
                .replace_shared(Some(path), Arc::clone(&staged.text))
                .map_err(|error| anyhow!("invalid source {path}: {error}"))?;
        }

        self.staged = files;
        Ok(sources.snapshot())
    }

    /// Turn one pipeline run into the report the host receives.
    fn collect(&mut self, run: PipelineRun) -> Result<CompileOutput> {
        let mut messages = self.host.take_messages();
        let log = self
            .processor
            .engine()
            .written_file(&self.log_name)
            .unwrap_or_default();
        let diagnostics = diagnostics::parse(
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

        let mut metrics = self.processor.sink().metrics();
        let tail_pending = self.processor.sink().tail_pending();
        let frame = self.processor.sink_mut().take();
        let published = frame.is_some();

        let raster = self.raster;
        let (pages, changed, removed, visuals) = match frame {
            Some(frame) => (
                frame
                    .pages_svg
                    .iter()
                    .filter(|_| !raster)
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

        let (rasters, raster_ms) = if published {
            self.rasterize_changed(&changed)?
        } else {
            (Vec::new(), 0.0)
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

        metrics.pages_rendered += rasters.len();
        metrics.render_ms += raster_ms;

        Ok(CompileOutput {
            published,
            pages,
            rasters,
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
            metrics,
        })
    }
}

/// One file's text as the session holds it.
#[derive(Clone)]
struct Staged {
    text: Arc<str>,
    /// Lets UTF-16 offsets be used as byte offsets without a scan.
    ascii: bool,
}

impl Staged {
    fn new(text: String) -> Self {
        let ascii = text.is_ascii();
        Self {
            text: Arc::from(text),
            ascii,
        }
    }

    /// The text with `edit` applied, or `None` when its range does not fit.
    fn splice(&self, edit: &SourceEdit) -> Option<Self> {
        if edit.from > edit.to {
            return None;
        }
        let (from, to) = if self.ascii {
            (edit.from, edit.to)
        } else {
            let from = byte_offset(&self.text, edit.from)?;
            (
                from,
                from + byte_offset(&self.text[from..], edit.to - edit.from)?,
            )
        };
        let before = self.text.get(..from)?;
        let after = self.text.get(to..)?;

        let mut text = String::with_capacity(before.len() + edit.insert.len() + after.len());
        text.push_str(before);
        text.push_str(&edit.insert);
        text.push_str(after);
        Some(Self {
            text: Arc::from(text),
            ascii: self.ascii && edit.insert.is_ascii(),
        })
    }
}

/// The byte offset `units` UTF-16 code units into `text`, if one falls there.
fn byte_offset(text: &str, units: usize) -> Option<usize> {
    let mut counted = 0;
    for (offset, character) in text.char_indices() {
        if counted >= units {
            return (counted == units).then_some(offset);
        }
        counted += character.len_utf16();
    }
    (counted == units).then_some(text.len())
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
        // SAFETY: called on the engines' thread, and `self.0` came from
        // pdf_files_suspend. Closing first discards whatever an aborted run
        // left installed.
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

#[cfg(test)]
mod tests {
    use super::{byte_offset, SourceEdit, Staged};

    fn edit(from: usize, to: usize, insert: &str) -> SourceEdit {
        SourceEdit {
            base: 0,
            from,
            to,
            insert: insert.into(),
        }
    }

    fn spliced(text: &str, from: usize, to: usize, insert: &str) -> Option<String> {
        Staged::new(text.into())
            .splice(&edit(from, to, insert))
            .map(|staged| staged.text.to_string())
    }

    #[test]
    fn splices_ascii_by_offset() {
        assert_eq!(spliced("abcdef", 2, 4, "XY").as_deref(), Some("abXYef"));
        assert_eq!(spliced("abc", 3, 3, "!").as_deref(), Some("abc!"));
    }

    #[test]
    fn splices_utf16_offsets_past_wide_characters() {
        // The emoji is two UTF-16 units and four UTF-8 bytes.
        assert_eq!(spliced("é😀x", 3, 4, "y").as_deref(), Some("é😀y"));
        assert_eq!(spliced("é😀x", 1, 3, "").as_deref(), Some("éx"));
    }

    #[test]
    fn refuses_ranges_that_do_not_fit() {
        assert_eq!(spliced("abc", 2, 1, ""), None);
        assert_eq!(spliced("abc", 1, 9, ""), None);
        // Inside a surrogate pair.
        assert_eq!(spliced("😀", 1, 2, ""), None);
    }

    #[test]
    fn non_ascii_insert_switches_to_scanning() {
        let staged = Staged::new("ab".into()).splice(&edit(1, 1, "😀")).unwrap();
        assert!(!staged.ascii);
        let next = staged.splice(&edit(3, 4, "c")).unwrap();
        assert_eq!(&*next.text, "a😀c");
    }

    #[test]
    fn byte_offset_counts_utf16_units() {
        assert_eq!(byte_offset("a😀b", 3), Some(5));
        assert_eq!(byte_offset("a😀b", 4), Some(6));
        assert_eq!(byte_offset("a😀b", 5), None);
    }
}
