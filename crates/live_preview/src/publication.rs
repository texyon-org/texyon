use std::{collections::BTreeMap, fmt, sync::Arc};

use texyon_preview_protocol::{OperationMetrics, UpdateOutcome};
use texyon_preview_render::{PageSvg, PageVisual};

use super::{FrameId, FramePhase, RequestSeq, SequenceExhausted, SourceRevision};

/// One page payload included in a frame patch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FramePage {
    /// Zero-based document page index.
    pub page_index: usize,
    /// Self-contained SVG markup.
    pub svg: Arc<str>,
}

/// Complete metadata and differential payload for one coherent viewer frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FramePatch {
    /// Strictly increasing process-local frame identity.
    pub frame_id: FrameId,
    /// Accepted source revision rendered by this frame.
    pub source_revision: SourceRevision,
    /// Latest request sequence applied before compilation.
    pub applied_request_seq: RequestSeq,
    /// Operation phase.
    pub phase: FramePhase,
    /// Engine operation result represented by this coherent frame.
    pub outcome: UpdateOutcome,
    /// Complete page visual metadata in document order.
    pub pages: Vec<PageVisual>,
    /// Added or changed page indices.
    pub changed: Vec<usize>,
    /// Removed page indices from the previous belief.
    pub removed: Vec<usize>,
    /// SVG payload for every index in `changed`, in the same order.
    pub pages_svg: Vec<FramePage>,
}

/// Identity and outcome shared by every page of one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameHeader {
    /// Accepted source revision rendered by the frame.
    pub source_revision: SourceRevision,
    /// Latest request sequence applied before compilation.
    pub applied_request_seq: RequestSeq,
    /// Operation phase.
    pub phase: FramePhase,
    /// Engine operation result represented by the frame.
    pub outcome: UpdateOutcome,
}

/// Transport boundary that must accept a whole frame atomically.
pub trait FrameSink {
    /// Sink-specific publication failure.
    type Error;

    /// Configure metrics copied into the next atomic frame, when supported.
    fn set_operation_metrics(&mut self, _metrics: OperationMetrics) {}

    /// Configure whether the next atomic frame leaves background tail work.
    fn set_tail_pending(&mut self, _tail_pending: bool) {}

    /// Accept one complete frame for transport.
    fn publish(&mut self, frame: &FramePatch) -> Result<(), Self::Error>;
}

/// Server-side belief about one retained client page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageBelief {
    /// Renderer-owned page identity and metadata.
    pub visual: PageVisual,
    /// Last successfully published SVG payload.
    pub svg: Arc<str>,
}

/// Server-side ledger of the complete coherent state believed to be retained.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientBelief {
    pages: Vec<PageBelief>,
    source_revision: Option<SourceRevision>,
    outcome: Option<UpdateOutcome>,
}

impl ClientBelief {
    /// Page beliefs in document order.
    pub fn pages(&self) -> &[PageBelief] {
        &self.pages
    }

    /// Source revision represented by the retained payloads.
    pub fn source_revision(&self) -> Option<SourceRevision> {
        self.source_revision
    }

    pub(crate) fn changed_pages(&self, visuals: &[PageVisual]) -> Vec<usize> {
        visuals
            .iter()
            .enumerate()
            .filter_map(|(index, visual)| {
                (self.pages.get(index).map(|page| &page.visual) != Some(visual)).then_some(index)
            })
            .collect()
    }
}

/// Failure to construct or publish a coherent frame.
#[derive(Debug)]
pub enum PublicationError<E> {
    /// Frame identity space has been exhausted.
    Sequence(SequenceExhausted),
    /// Render output does not exactly cover the changed-page set.
    InvalidRenderSet(String),
    /// Page metadata cannot be represented safely by the viewer.
    InvalidPageMetadata {
        /// Zero-based page index containing invalid metadata.
        page_index: usize,
        /// Stable validation failure description.
        reason: &'static str,
    },
    /// The transport rejected the complete event.
    Sink(E),
    /// No coherent frame has been published for resynchronization yet.
    NoBelief,
}

impl<E: fmt::Display> fmt::Display for PublicationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sequence(error) => error.fmt(f),
            Self::InvalidRenderSet(message) => write!(f, "invalid rendered page set: {message}"),
            Self::InvalidPageMetadata { page_index, reason } => {
                write!(f, "invalid metadata for page {page_index}: {reason}")
            }
            Self::Sink(error) => write!(f, "frame publication failed: {error}"),
            Self::NoBelief => write!(f, "no published viewer belief is available"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for PublicationError<E> {}

/// Transactional owner of frame identities and server-side client belief.
#[derive(Clone, Debug, Default)]
pub struct PreviewPublisher {
    belief: ClientBelief,
    last_frame_id: FrameId,
}

impl PreviewPublisher {
    /// Construct an empty publisher. The first accepted frame receives ID one.
    pub fn new() -> Self {
        Self::default()
    }

    /// Last frame successfully accepted by the transport.
    pub fn last_frame_id(&self) -> FrameId {
        self.last_frame_id
    }

    /// Current server-side belief, committed only after sink acceptance.
    pub fn belief(&self) -> &ClientBelief {
        &self.belief
    }

    /// Construct and publish a differential update transactionally.
    pub fn publish<S: FrameSink>(
        &mut self,
        sink: &mut S,
        header: FrameHeader,
        visuals: &[PageVisual],
        rendered: Vec<PageSvg>,
    ) -> Result<FrameId, PublicationError<S::Error>> {
        validate_visuals::<S::Error>(visuals)?;
        let changed = self.belief.changed_pages(visuals);
        let removed = (visuals.len()..self.belief.pages.len()).collect::<Vec<_>>();
        let rendered = index_rendered::<S::Error>(rendered, visuals.len())?;

        let supplied = rendered.keys().copied().collect::<Vec<_>>();
        if supplied != changed {
            return Err(PublicationError::InvalidRenderSet(format!(
                "expected pages {changed:?}, received {supplied:?}"
            )));
        }

        let mut candidate = Vec::with_capacity(visuals.len());
        let mut pages_svg = Vec::with_capacity(changed.len());
        for (page_index, visual) in visuals.iter().cloned().enumerate() {
            let svg = if let Some(svg) = rendered.get(&page_index) {
                let svg: Arc<str> = Arc::from(svg.as_str());
                pages_svg.push(FramePage {
                    page_index,
                    svg: Arc::clone(&svg),
                });
                svg
            } else {
                Arc::clone(&self.belief.pages[page_index].svg)
            };
            candidate.push(PageBelief { visual, svg });
        }

        let frame_id = self
            .last_frame_id
            .checked_next()
            .map_err(PublicationError::Sequence)?;
        let patch = FramePatch {
            frame_id,
            source_revision: header.source_revision,
            applied_request_seq: header.applied_request_seq,
            phase: header.phase,
            outcome: header.outcome,
            pages: visuals.to_vec(),
            changed,
            removed,
            pages_svg,
        };
        sink.publish(&patch).map_err(PublicationError::Sink)?;

        self.belief.pages = candidate;
        self.belief.source_revision = Some(header.source_revision);
        self.belief.outcome = Some(header.outcome);
        self.last_frame_id = frame_id;
        Ok(frame_id)
    }

    /// Republish every retained payload after a client loses its local state.
    pub fn resync<S: FrameSink>(
        &mut self,
        sink: &mut S,
        applied_request_seq: RequestSeq,
    ) -> Result<FrameId, PublicationError<S::Error>> {
        let source_revision = self
            .belief
            .source_revision
            .ok_or(PublicationError::NoBelief)?;
        let outcome = self.belief.outcome.ok_or(PublicationError::NoBelief)?;
        let frame_id = self
            .last_frame_id
            .checked_next()
            .map_err(PublicationError::Sequence)?;
        let patch = FramePatch {
            frame_id,
            source_revision,
            applied_request_seq,
            phase: FramePhase::Resync,
            outcome,
            pages: self
                .belief
                .pages
                .iter()
                .map(|page| page.visual.clone())
                .collect(),
            changed: (0..self.belief.pages.len()).collect(),
            removed: Vec::new(),
            pages_svg: self
                .belief
                .pages
                .iter()
                .enumerate()
                .map(|(page_index, page)| FramePage {
                    page_index,
                    svg: Arc::clone(&page.svg),
                })
                .collect(),
        };
        sink.publish(&patch).map_err(PublicationError::Sink)?;
        self.last_frame_id = frame_id;
        Ok(frame_id)
    }
}

fn validate_visuals<E>(visuals: &[PageVisual]) -> Result<(), PublicationError<E>> {
    for (page_index, visual) in visuals.iter().enumerate() {
        if !visual.size_bp[0].is_finite() || !visual.size_bp[1].is_finite() {
            return Err(PublicationError::InvalidPageMetadata {
                page_index,
                reason: "non-finite page size",
            });
        }
        if visual.size_bp[0] <= 0.0 || visual.size_bp[1] <= 0.0 {
            return Err(PublicationError::InvalidPageMetadata {
                page_index,
                reason: "non-positive page size",
            });
        }
    }
    Ok(())
}

fn index_rendered<E>(
    rendered: Vec<PageSvg>,
    page_count: usize,
) -> Result<BTreeMap<usize, String>, PublicationError<E>> {
    let mut indexed = BTreeMap::new();
    for page in rendered {
        if page.page_index >= page_count {
            return Err(PublicationError::InvalidRenderSet(format!(
                "page {} is outside page count {page_count}",
                page.page_index
            )));
        }
        let page_index = page.page_index;
        if indexed.insert(page_index, page.svg).is_some() {
            return Err(PublicationError::InvalidRenderSet(format!(
                "page {page_index} was rendered more than once"
            )));
        }
    }
    Ok(indexed)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use texyon_preview_render::PageFingerprint;

    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        frames: Vec<FramePatch>,
        reject: bool,
    }

    impl FrameSink for RecordingSink {
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

    fn visual(byte: u8) -> PageVisual {
        PageVisual {
            fingerprint: PageFingerprint::Stable([byte; 32]),
            size_bp: [612.0, 792.0],
            links: Vec::new(),
            destinations: Vec::new(),
            unsupported: BTreeMap::new(),
        }
    }

    fn header(source_revision: SourceRevision, applied_request_seq: RequestSeq) -> FrameHeader {
        FrameHeader {
            source_revision,
            applied_request_seq,
            phase: FramePhase::Foreground,
            outcome: UpdateOutcome::Complete,
        }
    }

    fn svg(page_index: usize, text: &str) -> PageSvg {
        PageSvg {
            page_index,
            svg: text.into(),
        }
    }

    #[test]
    fn publishes_exact_changed_set_and_removals() {
        let mut publisher = PreviewPublisher::new();
        let mut sink = RecordingSink::default();
        publisher
            .publish(
                &mut sink,
                header(SourceRevision::ZERO, RequestSeq::ZERO),
                &[visual(1), visual(2), visual(3)],
                vec![svg(0, "a"), svg(1, "b"), svg(2, "c")],
            )
            .unwrap();
        publisher
            .publish(
                &mut sink,
                header(SourceRevision::new(1), RequestSeq::new(1)),
                &[visual(1), visual(9)],
                vec![svg(1, "changed")],
            )
            .unwrap();

        let patch = sink.frames.last().unwrap();
        assert_eq!(patch.changed, vec![1]);
        assert_eq!(patch.removed, vec![2]);
        assert_eq!(patch.pages_svg[0].svg.as_ref(), "changed");
        assert_eq!(publisher.belief().pages()[0].svg.as_ref(), "a");
        assert_eq!(publisher.belief().pages()[1].svg.as_ref(), "changed");
    }

    #[test]
    fn metadata_only_frame_still_publishes() {
        let mut publisher = PreviewPublisher::new();
        let mut sink = RecordingSink::default();
        publisher
            .publish(
                &mut sink,
                header(SourceRevision::ZERO, RequestSeq::ZERO),
                &[visual(1)],
                vec![svg(0, "a")],
            )
            .unwrap();
        let frame_id = publisher
            .publish(
                &mut sink,
                header(SourceRevision::new(1), RequestSeq::new(1)),
                &[visual(1)],
                vec![],
            )
            .unwrap();

        assert_eq!(frame_id, FrameId::new(2));
        assert!(sink.frames[1].changed.is_empty());
        assert!(sink.frames[1].pages_svg.is_empty());
        assert_eq!(sink.frames[1].source_revision, SourceRevision::new(1));
    }

    #[test]
    fn sink_failure_leaves_belief_and_frame_identity_uncommitted() {
        let mut publisher = PreviewPublisher::new();
        let mut sink = RecordingSink {
            reject: true,
            ..RecordingSink::default()
        };

        assert!(matches!(
            publisher.publish(
                &mut sink,
                header(SourceRevision::ZERO, RequestSeq::ZERO),
                &[visual(1)],
                vec![svg(0, "a")],
            ),
            Err(PublicationError::Sink("closed"))
        ));
        assert!(publisher.belief().pages().is_empty());
        assert_eq!(publisher.last_frame_id(), FrameId::ZERO);
    }

    #[test]
    fn refuses_partial_duplicate_and_extraneous_render_output() {
        let cases = [
            vec![svg(0, "a")],
            vec![svg(0, "a"), svg(0, "again"), svg(1, "b")],
            vec![svg(0, "a"), svg(1, "b"), svg(2, "outside")],
        ];

        for rendered in cases {
            let mut publisher = PreviewPublisher::new();
            let mut sink = RecordingSink::default();
            assert!(matches!(
                publisher.publish(
                    &mut sink,
                    header(SourceRevision::ZERO, RequestSeq::ZERO),
                    &[visual(1), visual(2)],
                    rendered,
                ),
                Err(PublicationError::InvalidRenderSet(_))
            ));
            assert!(sink.frames.is_empty());
        }
    }

    #[test]
    fn resync_republishes_every_retained_page_under_new_frame() {
        let mut publisher = PreviewPublisher::new();
        let mut sink = RecordingSink::default();
        publisher
            .publish(
                &mut sink,
                header(SourceRevision::new(4), RequestSeq::ZERO),
                &[visual(1), visual(2)],
                vec![svg(0, "a"), svg(1, "b")],
            )
            .unwrap();
        publisher.resync(&mut sink, RequestSeq::new(7)).unwrap();

        let patch = sink.frames.last().unwrap();
        assert_eq!(patch.frame_id, FrameId::new(2));
        assert_eq!(patch.phase, FramePhase::Resync);
        assert_eq!(patch.source_revision, SourceRevision::new(4));
        assert_eq!(patch.applied_request_seq, RequestSeq::new(7));
        assert_eq!(patch.changed, vec![0, 1]);
        assert_eq!(patch.pages_svg.len(), 2);
    }

    #[test]
    fn resync_requires_a_published_belief() {
        let mut publisher = PreviewPublisher::new();
        let mut sink = RecordingSink::default();
        assert!(matches!(
            publisher.resync(&mut sink, RequestSeq::ZERO),
            Err(PublicationError::NoBelief)
        ));
        assert!(sink.frames.is_empty());
    }
}
