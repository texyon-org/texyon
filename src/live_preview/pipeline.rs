use std::{fmt, sync::Arc, time::Instant};

use tectonic_preview_protocol::{OperationMetrics, UpdateOutcome};
use tectonic_preview_render::{Backend, HotRenderer, RenderError, ResourceProvider};

use crate::engines::tex::SyncIndex;

use super::{
    AssembledDocument, DocumentAssembler, DocumentFragment, FrameId, FramePhase, FrameSink,
    PreviewPublisher, PublicationError, RequestSeq, SourceRevision, XdvAssemblyError,
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
///
/// This is the Milestone 6 rung-zero path. Fence-local fragments can enter
/// through the same assembler later without changing publication semantics.
pub struct PreviewPipeline<P, B> {
    assembler: DocumentAssembler,
    renderer: HotRenderer<P>,
    backend: B,
    publisher: PreviewPublisher,
    last_document: Option<AssembledDocument>,
    sync_index: SyncIndex,
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
        }
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
        self.process_fragment_with_engine_ms(
            sink,
            fragment,
            source_revision,
            applied_request_seq,
            phase,
            outcome,
            OperationMetrics::default(),
            false,
            SyncIndex::default(),
        )
    }

    pub(crate) fn process_fragment_with_engine_ms<S: FrameSink>(
        &mut self,
        sink: &mut S,
        fragment: DocumentFragment<'_>,
        source_revision: SourceRevision,
        applied_request_seq: RequestSeq,
        phase: FramePhase,
        outcome: UpdateOutcome,
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
            .analyze(Arc::clone(document.bytes()))
            .map_err(PreviewPipelineError::Render)?;
        let analysis_ms = analysis_started.elapsed().as_secs_f64() * 1000.0;
        metrics.analysis_ms = analysis_ms;
        metrics.pages_fingerprinted = analysis.pages().len();
        metrics.unsupported_pages = analysis
            .pages()
            .iter()
            .filter(|page| !page.unsupported.is_empty())
            .count();
        let covered_pages = analysis.pages().len();
        let sync_index = if preserve_tail_belief {
            sync_index.with_preserved_tail(&self.sync_index, covered_pages)
        } else {
            sync_index
        };
        let mut visuals = analysis.pages().to_vec();
        if preserve_tail_belief && self.publisher.belief().pages().len() > visuals.len() {
            visuals.extend(
                self.publisher.belief().pages()[visuals.len()..]
                    .iter()
                    .map(|page| page.visual.clone()),
            );
        }
        let selected = visuals
            .iter()
            .enumerate()
            .filter_map(|(page_index, visual)| {
                (self
                    .publisher
                    .belief()
                    .pages()
                    .get(page_index)
                    .map(|page| &page.visual)
                    != Some(visual))
                .then_some(page_index)
            })
            .collect::<Vec<_>>();
        let render_started = Instant::now();
        let rendered = self
            .renderer
            .render_pages(&analysis, &selected, &mut self.backend)
            .map_err(PreviewPipelineError::Render)?;
        let render_ms = render_started.elapsed().as_secs_f64() * 1000.0;
        metrics.render_ms = render_ms;
        metrics.pages_rendered = rendered.len();
        sink.set_operation_metrics(metrics);
        let frame_id = self
            .publisher
            .publish(
                sink,
                source_revision,
                applied_request_seq,
                phase,
                outcome,
                &visuals,
                rendered,
            )
            .map_err(PreviewPipelineError::Publication)?;

        self.last_document = Some(document);
        self.sync_index = sync_index;
        Ok(frame_id)
    }

    /// Absorb a completed-but-superseded operation's XDV without analysis,
    /// rendering, or publication.
    ///
    /// The engine has already committed the operation, so the assembled
    /// document chain must advance for the next fragment's splice to remain
    /// coherent; the viewer belief and sync index stay untouched, matching the
    /// protocol's promise that superseded work never replaces client state.
    pub(crate) fn absorb_fragment(
        &mut self,
        fragment: DocumentFragment<'_>,
    ) -> Result<(), XdvAssemblyError> {
        let document = self.assembler.assemble(self.last_document.as_ref(), fragment)?;
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
    use tectonic_preview_render::{
        NullBackend, Resource, ResourceKind, ResourceProvider, ResourceRequest,
    };

    use super::*;
    use crate::live_preview::FramePatch;

    const ONE_PAGE: &[u8] = include_bytes!("../../tests/tex-outputs/the_letter_a.xdv");

    struct ClassicResources;

    impl ResourceProvider for ClassicResources {
        fn resolve(&self, request: &ResourceRequest) -> Result<Option<Resource>, RenderError> {
            let bytes: &[u8] = match request.kind {
                ResourceKind::Tfm => include_bytes!("../../tests/assets/cmr10.tfm"),
                ResourceKind::FontProgram => include_bytes!("../../tests/assets/cmr10.pfb"),
                ResourceKind::Encoding => return Ok(None),
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
            tectonic_preview_render::MupdfBackend::new().unwrap(),
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
        crate::test_util::activate_test_mode_augmented(env!("CARGO_MANIFEST_DIR"));
        let bundle = crate::live_preview::FrozenBundle::new(Box::new(
            crate::test_util::TestBundle::default(),
        ));
        let provider = crate::live_preview::BundleResourceProvider::new(bundle, None).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/tex-outputs");
        let mut backend = tectonic_preview_render::MupdfBackend::new().unwrap();
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("xdv") {
                continue;
            }
            let mut renderer = tectonic_preview_render::HotRenderer::new(provider.clone());
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
