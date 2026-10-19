use std::{fmt, sync::Arc, time::Instant};

use crate::processor::elapsed_ms;

use texyon_preview_protocol::{OperationMetrics, UpdateOutcome};
use texyon_preview_render::{Backend, HotRenderer, PageSvg, RenderError, ResourceProvider};

use texyon_engine_xetex::SyncIndex;

use super::{
    AssembledDocument, DocumentAssembler, DocumentFragment, FrameHeader, FrameId, FramePhase,
    FrameSink, PreviewPublisher, PublicationError, RequestSeq, SourceRevision, XdvAssemblyError,
};

/// Assembly, analysis/render, or atomic publication failure.
#[derive(Debug)]
pub enum PreviewPipelineError<E> {
    /// XDV structural validation failed.
    Assembly(XdvAssemblyError),
    /// Portable analysis or backend rendering failed.
    Render(RenderError),
    /// A coherent patch could not be constructed or accepted by the sink.
    Publication(PublicationError<E>),
}

impl<E: fmt::Display> fmt::Display for PreviewPipelineError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assembly(error) => error.fmt(f),
            Self::Render(error) => error.fmt(f),
            Self::Publication(error) => error.fmt(f),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for PreviewPipelineError<E> {}

/// Stateful full-document analysis, differential render, and publication path.
pub struct PreviewPipeline<P, B> {
    assembler: DocumentAssembler,
    renderer: HotRenderer<P>,
    backend: B,
    publisher: PreviewPublisher,
    last_document: Option<AssembledDocument>,
    sync_index: SyncIndex,
    /// Whether frames carry page SVG, or the host draws pages with [`Self::rasterize`].
    payloads: bool,
}

impl<P: ResourceProvider, B: Backend> PreviewPipeline<P, B> {
    /// Construct an empty pipeline around frozen resource and drawing adapters.
    pub fn new(provider: P, backend: B) -> Self {
        Self {
            assembler: DocumentAssembler::new(),
            renderer: HotRenderer::new(provider),
            backend,
            publisher: PreviewPublisher::new(),
            last_document: None,
            sync_index: SyncIndex::default(),
            payloads: true,
        }
    }

    /// Publish frames without page payloads. The host asks for the pages it shows.
    pub fn draw_on_demand(&mut self) {
        self.payloads = false;
    }

    /// Draw published pages to RGBA at `scale` pixels per bp, in the backend's raster layout.
    pub fn rasterize(
        &mut self,
        page_indices: &[usize],
        scale: f32,
    ) -> Result<Vec<(usize, Vec<u8>)>, RenderError> {
        let published = self.publisher.belief().pages().len();
        if let Some(page_index) = page_indices.iter().find(|&&index| index >= published) {
            return Err(RenderError::InvalidSelection(format!(
                "page {page_index} exceeds page count {published}"
            )));
        }
        self.backend.set_raster(Some(scale))?;
        let rendered = self
            .renderer
            .render_retained(page_indices, &mut self.backend);
        self.backend.set_raster(None)?;
        rendered
    }

    /// Last document whose complete differential frame reached the sink.
    pub fn last_document(&self) -> Option<&AssembledDocument> {
        self.last_document.as_ref()
    }

    /// Transactional server-side viewer belief.
    pub fn publisher(&self) -> &PreviewPublisher {
        &self.publisher
    }

    /// Source geometry committed with the current viewer belief.
    pub fn sync_index(&self) -> &SyncIndex {
        &self.sync_index
    }

    /// Validate and publish one complete XDV/SPX operation.
    pub fn process_complete<S: FrameSink>(
        &mut self,
        sink: &mut S,
        xdv: &[u8],
        source_revision: SourceRevision,
        applied_request_seq: RequestSeq,
        phase: FramePhase,
        outcome: UpdateOutcome,
    ) -> Result<FrameId, PreviewPipelineError<S::Error>> {
        self.process_fragment(
            sink,
            DocumentFragment::Complete(xdv),
            source_revision,
            applied_request_seq,
            phase,
            outcome,
        )
    }

    /// Assemble and publish a complete or absolute-offset tail fragment.
    pub fn process_fragment<S: FrameSink>(
        &mut self,
        sink: &mut S,
        fragment: DocumentFragment<'_>,
        source_revision: SourceRevision,
        applied_request_seq: RequestSeq,
        phase: FramePhase,
        outcome: UpdateOutcome,
    ) -> Result<FrameId, PreviewPipelineError<S::Error>> {
        self.publish_fragment(
            sink,
            fragment,
            FrameHeader {
                source_revision,
                applied_request_seq,
                phase,
                outcome,
            },
            OperationMetrics::default(),
            false,
            SyncIndex::default(),
        )
    }

    pub(crate) fn publish_fragment<S: FrameSink>(
        &mut self,
        sink: &mut S,
        fragment: DocumentFragment<'_>,
        header: FrameHeader,
        mut metrics: OperationMetrics,
        preserve_tail_belief: bool,
        sync_index: SyncIndex,
    ) -> Result<FrameId, PreviewPipelineError<S::Error>> {
        let document = self
            .assembler
            .assemble(self.last_document.as_ref(), fragment)
            .map_err(PreviewPipelineError::Assembly)?;
        let analysis_started = Instant::now();
        let analysis = self
            .renderer
            .analyze_from(Arc::clone(document.bytes()), document.page_offsets())
            .map_err(PreviewPipelineError::Render)?;
        metrics.analysis_ms = elapsed_ms(analysis_started);
        if !preserve_tail_belief {
            self.renderer.release_stale_plans();
        }
        metrics.pages_fingerprinted = analysis.pages().len();
        metrics.unsupported_pages = analysis
            .pages()
            .iter()
            .filter(|page| !page.unsupported.is_empty())
            .count();
        let sync_index = if preserve_tail_belief {
            sync_index.with_preserved_tail(&self.sync_index, analysis.pages().len())
        } else {
            sync_index
        };
        let mut visuals = analysis.pages().to_vec();
        let believed = self.publisher.belief().pages();
        if preserve_tail_belief && believed.len() > visuals.len() {
            visuals.extend(
                believed[visuals.len()..]
                    .iter()
                    .map(|page| page.visual.clone()),
            );
        }
        let selected = self.publisher.belief().changed_pages(&visuals);
        let render_started = Instant::now();
        let rendered = if self.payloads {
            self.renderer
                .render_pages(&analysis, &selected, &mut self.backend)
                .map_err(PreviewPipelineError::Render)?
        } else {
            self.renderer.commit(&analysis);
            selected
                .iter()
                .map(|&page_index| PageSvg {
                    page_index,
                    svg: String::new(),
                })
                .collect()
        };
        metrics.render_ms = elapsed_ms(render_started);
        metrics.pages_rendered = if self.payloads { rendered.len() } else { 0 };
        sink.set_operation_metrics(metrics);
        let frame_id = self
            .publisher
            .publish(sink, header, &visuals, rendered)
            .map_err(PreviewPipelineError::Publication)?;

        self.last_document = Some(document);
        self.sync_index = sync_index;
        Ok(frame_id)
    }

    /// Advance the assembled document for an operation the engine committed
    /// but that was superseded, leaving viewer belief and sync index untouched.
    pub(crate) fn absorb_fragment(
        &mut self,
        fragment: DocumentFragment<'_>,
    ) -> Result<(), XdvAssemblyError> {
        let document = self
            .assembler
            .assemble(self.last_document.as_ref(), fragment)?;
        self.last_document = Some(document);
        Ok(())
    }

    /// Republish the complete retained belief without engine or renderer work.
    pub fn resync<S: FrameSink>(
        &mut self,
        sink: &mut S,
        applied_request_seq: RequestSeq,
    ) -> Result<FrameId, PreviewPipelineError<S::Error>> {
        sink.set_operation_metrics(OperationMetrics::default());
        self.publisher
            .resync(sink, applied_request_seq)
            .map_err(PreviewPipelineError::Publication)
    }
}

#[cfg(test)]
mod tests {
    use texyon_preview_render::{
        NullBackend, Resource, ResourceKind, ResourceProvider, ResourceRequest,
    };

    use super::*;
    use crate::FramePatch;

    const ONE_PAGE: &[u8] = include_bytes!("../../../tests/tex-outputs/the_letter_a.xdv");

    struct ClassicResources;

    impl ResourceProvider for ClassicResources {
        fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
            let bytes: &[u8] = match request.kind {
                ResourceKind::Tfm => include_bytes!("../../../tests/assets/cmr10.tfm"),
                ResourceKind::FontProgram => include_bytes!("../../../tests/assets/cmr10.pfb"),
                _ => return Ok(None),
            };
            Ok(Some(Resource::from_bytes(bytes)))
        }
    }

    #[derive(Default)]
    struct Sink {
        frames: Vec<FramePatch>,
        metrics: Vec<OperationMetrics>,
        reject: bool,
    }

    impl FrameSink for Sink {
        type Error = &'static str;

        fn set_operation_metrics(&mut self, metrics: OperationMetrics) {
            self.metrics.push(metrics);
        }

        fn publish(&mut self, frame: &FramePatch) -> Result<(), Self::Error> {
            if self.reject {
                Err("closed")
            } else {
                self.frames.push(frame.clone());
                Ok(())
            }
        }
    }

    #[test]
    fn full_document_flows_through_analysis_render_and_publication() {
        let mut pipeline = PreviewPipeline::new(ClassicResources, NullBackend);
        let mut sink = Sink::default();
        let first = pipeline
            .process_complete(
                &mut sink,
                ONE_PAGE,
                SourceRevision::ZERO,
                RequestSeq::ZERO,
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            )
            .unwrap();
        let second = pipeline
            .process_complete(
                &mut sink,
                ONE_PAGE,
                SourceRevision::new(1),
                RequestSeq::new(1),
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            )
            .unwrap();

        assert_eq!(first, FrameId::new(1));
        assert_eq!(second, FrameId::new(2));
        assert_eq!(sink.frames[0].changed, vec![0]);
        assert!(sink.frames[1].changed.is_empty());
        assert!(sink.frames[1].pages_svg.is_empty());
        assert_eq!(pipeline.last_document().unwrap().bytes().as_ref(), ONE_PAGE);
    }

    #[cfg(all(feature = "preview-mupdf", target_os = "linux"))]
    #[test]
    fn full_document_publishes_real_path_svg() {
        let mut pipeline = PreviewPipeline::new(
            ClassicResources,
            texyon_preview_render::MupdfBackend::new().unwrap(),
        );
        let mut sink = Sink::default();
        pipeline
            .process_complete(
                &mut sink,
                ONE_PAGE,
                SourceRevision::ZERO,
                RequestSeq::ZERO,
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            )
            .unwrap();

        assert_eq!(sink.frames.len(), 1);
        assert_eq!(sink.frames[0].changed, vec![0]);
        assert_eq!(sink.frames[0].pages_svg.len(), 1);
        assert!(sink.frames[0].pages_svg[0].svg.contains("<path"));
        assert_eq!(sink.metrics.len(), 1);
        assert_eq!(sink.metrics[0].engine_ms, 0.0);
        assert!(sink.metrics[0].analysis_ms > 0.0);
        assert!(sink.metrics[0].render_ms > 0.0);
    }

    #[cfg(all(feature = "preview-mupdf", target_os = "linux"))]
    #[test]
    fn xdv_fixture_corpus_is_fully_renderable() {
        let bundle =
            crate::FrozenBundle::new(Box::new(crate::test_support::AssetBundle::default()));
        let provider = crate::BundleResourceProvider::new(bundle, None).unwrap();
        let root = crate::test_support::asset_path(&["tex-outputs"]);
        let mut backend = texyon_preview_render::MupdfBackend::new().unwrap();
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("xdv") {
                continue;
            }
            // Metric-only test TFMs have no glyph outlines to draw.
            let source = std::fs::read_to_string(path.with_extension("tex")).unwrap();
            if source.contains("mcta") || source.contains("mctb") {
                continue;
            }
            let mut renderer = texyon_preview_render::HotRenderer::new(provider.clone());
            let analysis = renderer
                .analyze(std::sync::Arc::from(std::fs::read(&path).unwrap()))
                .unwrap();
            assert!(
                analysis.ledger().unsupported().is_empty(),
                "{}: {:?}",
                path.display(),
                analysis.ledger().unsupported()
            );
            let selected: Vec<_> = (0..analysis.pages().len()).collect();
            let pages = renderer
                .render_pages(&analysis, &selected, &mut backend)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(pages.iter().all(|page| page.svg.contains("<svg")));
        }
    }

    #[test]
    fn failures_retain_last_published_document_and_belief() {
        let mut pipeline = PreviewPipeline::new(ClassicResources, NullBackend);
        let mut sink = Sink::default();
        pipeline
            .process_complete(
                &mut sink,
                ONE_PAGE,
                SourceRevision::ZERO,
                RequestSeq::ZERO,
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            )
            .unwrap();
        let previous = Arc::clone(pipeline.last_document().unwrap().bytes());
        let previous_belief = pipeline.publisher().belief().clone();
        let mut corrupt = ONE_PAGE.to_vec();
        corrupt[0] = 0;

        assert!(matches!(
            pipeline.process_complete(
                &mut sink,
                &corrupt,
                SourceRevision::new(1),
                RequestSeq::new(1),
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            ),
            Err(PreviewPipelineError::Assembly(_))
        ));
        assert_eq!(pipeline.last_document().unwrap().bytes(), &previous);
        assert_eq!(pipeline.publisher().belief(), &previous_belief);

        sink.reject = true;
        assert!(matches!(
            pipeline.process_complete(
                &mut sink,
                ONE_PAGE,
                SourceRevision::new(1),
                RequestSeq::new(1),
                FramePhase::Foreground,
                UpdateOutcome::Complete,
            ),
            Err(PreviewPipelineError::Publication(PublicationError::Sink(
                "closed"
            )))
        ));
        assert_eq!(pipeline.last_document().unwrap().bytes(), &previous);
        assert_eq!(pipeline.publisher().belief(), &previous_belief);
    }
}
