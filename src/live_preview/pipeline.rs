use std::{fmt, sync::Arc};

use tectonic_preview_protocol::UpdateOutcome;
use tectonic_preview_render::{Backend, HotRenderer, RenderError, ResourceProvider};

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
        let document = self
            .assembler
            .assemble(self.last_document.as_ref(), fragment)
            .map_err(PreviewPipelineError::Assembly)?;
        let analysis = self
            .renderer
            .analyze(Arc::clone(document.bytes()))
            .map_err(PreviewPipelineError::Render)?;
        let selected = analysis
            .pages()
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
        let rendered = self
            .renderer
            .render_pages(&analysis, &selected, &mut self.backend)
            .map_err(PreviewPipelineError::Render)?;
        let frame_id = self
            .publisher
            .publish(
                sink,
                source_revision,
                applied_request_seq,
                phase,
                outcome,
                analysis.pages(),
                rendered,
            )
            .map_err(PreviewPipelineError::Publication)?;

        self.last_document = Some(document);
        Ok(frame_id)
    }

    /// Republish the complete retained belief without engine or renderer work.
    pub fn resync<S: FrameSink>(
        &mut self,
        sink: &mut S,
        source_revision: SourceRevision,
        applied_request_seq: RequestSeq,
    ) -> Result<FrameId, PreviewPipelineError<S::Error>> {
        self.publisher
            .resync(sink, source_revision, applied_request_seq)
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
        reject: bool,
    }

    impl FrameSink for Sink {
        type Error = &'static str;

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
