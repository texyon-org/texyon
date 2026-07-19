use std::{fmt, io::Write};

use tectonic_preview_protocol::{
    encode_event, OperationMetrics, PageSize, PageSvgPayload, PageUnsupportedPayload,
    ProtocolError, ProtocolLimits, ServerEvent,
};

use super::{FramePatch, FrameSink};

/// JSONL encoding or output failure at the atomic frame sink.
#[derive(Debug)]
pub enum TransportError {
    /// Protocol encoding or frame-size validation failed.
    Protocol(ProtocolError),
    /// Output stream failed; callers must terminate or resynchronize.
    Io(std::io::Error),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(f),
            Self::Io(error) => write!(f, "preview transport write failed: {error}"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

/// Protocol-only JSONL writer used as the publication commit boundary.
pub struct JsonlFrameSink<W> {
    writer: W,
    limits: ProtocolLimits,
    metrics: OperationMetrics,
    tail_pending: bool,
}

impl<W: Write> JsonlFrameSink<W> {
    /// Construct a sink with zeroed operation metrics.
    pub fn new(writer: W, limits: ProtocolLimits) -> Self {
        Self {
            writer,
            limits,
            metrics: OperationMetrics::default(),
            tail_pending: false,
        }
    }

    /// Set operation metrics copied into the next published frame.
    pub fn set_metrics(&mut self, metrics: OperationMetrics) {
        self.metrics = metrics;
    }

    /// Set whether the next frame has a pending tail-settle operation.
    pub fn set_tail_pending(&mut self, tail_pending: bool) {
        self.tail_pending = tail_pending;
    }

    /// Encode and write a non-frame server event.
    pub fn write_event(&mut self, event: &ServerEvent) -> Result<(), TransportError> {
        let encoded = encode_event(event, self.limits).map_err(TransportError::Protocol)?;
        self.writer
            .write_all(&encoded)
            .map_err(TransportError::Io)?;
        self.writer.flush().map_err(TransportError::Io)
    }

    /// Access the underlying writer.
    pub fn writer(&self) -> &W {
        &self.writer
    }

    /// Consume the sink and return its writer.
    pub fn into_writer(self) -> W {
        self.writer
    }

    fn encode_frame(&self, frame: &FramePatch) -> Result<Vec<u8>, TransportError> {
        let mut metrics = self.metrics;
        let mut previous_size = usize::MAX;
        for _ in 0..4 {
            let event = ServerEvent::Update {
                frame_id: frame.frame_id,
                source_revision: frame.source_revision,
                applied_request_seq: frame.applied_request_seq,
                phase: frame.phase,
                outcome: frame.outcome,
                page_count: frame.pages.len(),
                changed: frame.changed.clone(),
                removed: frame.removed.clone(),
                pages_svg: frame
                    .pages_svg
                    .iter()
                    .map(|page| PageSvgPayload {
                        page_index: page.page_index,
                        svg: page.svg.to_string(),
                    })
                    .collect(),
                page_sizes: frame
                    .pages
                    .iter()
                    .map(|page| PageSize {
                        width_bp: page.size_bp[0],
                        height_bp: page.size_bp[1],
                    })
                    .collect(),
                unsupported_pages: frame
                    .pages
                    .iter()
                    .enumerate()
                    .filter(|(_, page)| !page.unsupported.is_empty())
                    .map(|(page_index, page)| PageUnsupportedPayload {
                        page_index,
                        operations: page.unsupported.clone(),
                    })
                    .collect(),
                metrics,
                tail_pending: self.tail_pending,
            };
            let encoded = encode_event(&event, self.limits).map_err(TransportError::Protocol)?;
            let frame_size = encoded.len() - 1;
            if frame_size == previous_size || metrics.frame_bytes == frame_size {
                return Ok(encoded);
            }
            previous_size = frame_size;
            metrics.frame_bytes = frame_size;
        }
        Err(TransportError::Protocol(ProtocolError::FrameTooLarge {
            bytes: previous_size,
            limit: self.limits.max_frame_bytes,
        }))
    }
}

impl<W: Write> FrameSink for JsonlFrameSink<W> {
    type Error = TransportError;

    fn set_operation_metrics(&mut self, metrics: OperationMetrics) {
        self.metrics = metrics;
    }

    fn set_tail_pending(&mut self, tail_pending: bool) {
        self.tail_pending = tail_pending;
    }

    fn publish(&mut self, frame: &FramePatch) -> Result<(), Self::Error> {
        let encoded = self.encode_frame(frame)?;
        self.writer
            .write_all(&encoded)
            .map_err(TransportError::Io)?;
        self.writer.flush().map_err(TransportError::Io)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, io};

    use tectonic_preview_protocol::{
        FrameId, FramePhase, RequestSeq, SourceRevision, UpdateOutcome,
    };
    use tectonic_preview_render::{PageFingerprint, PageVisual};

    use super::*;
    use crate::live_preview::FramePage;

    fn patch(svg: &str) -> FramePatch {
        let mut unsupported = BTreeMap::new();
        unsupported.insert("missing-image-resource".into(), 1);
        FramePatch {
            frame_id: FrameId::new(9_007_199_254_740_993),
            source_revision: SourceRevision::new(3),
            applied_request_seq: RequestSeq::new(4),
            phase: FramePhase::Foreground,
            outcome: UpdateOutcome::Complete,
            pages: vec![PageVisual {
                fingerprint: PageFingerprint::Stable([1; 32]),
                size_bp: [612.0, 792.0],
                unsupported,
            }],
            changed: vec![0],
            removed: vec![],
            pages_svg: vec![FramePage {
                page_index: 0,
                svg: svg.into(),
            }],
        }
    }

    #[test]
    fn frame_sink_emits_one_physical_line_with_exact_metrics() {
        let mut sink = JsonlFrameSink::new(Vec::new(), ProtocolLimits::default());
        sink.publish(&patch("<svg>\n<path/>\n</svg>")).unwrap();
        let wire = sink.into_writer();

        assert_eq!(wire.iter().filter(|&&byte| byte == b'\n').count(), 1);
        let event: ServerEvent = serde_json::from_slice(&wire[..wire.len() - 1]).unwrap();
        let ServerEvent::Update {
            frame_id,
            metrics,
            pages_svg,
            unsupported_pages,
            ..
        } = event
        else {
            panic!("not an update")
        };
        assert_eq!(frame_id, FrameId::new(9_007_199_254_740_993));
        assert_eq!(metrics.frame_bytes, wire.len() - 1);
        assert!(pages_svg[0].svg.contains('\n'));
        assert_eq!(unsupported_pages[0].page_index, 0);
        assert_eq!(unsupported_pages[0].operations["missing-image-resource"], 1);
    }

    #[test]
    fn oversized_frame_is_rejected_without_writing_prefix() {
        let limits = ProtocolLimits {
            max_frame_bytes: 64,
            ..ProtocolLimits::default()
        };
        let mut sink = JsonlFrameSink::new(Vec::new(), limits);
        assert!(matches!(
            sink.publish(&patch(&"x".repeat(100))),
            Err(TransportError::Protocol(
                ProtocolError::FrameTooLarge { .. }
            ))
        ));
        assert!(sink.writer().is_empty());
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn output_failure_surfaces_as_transport_failure() {
        let mut sink = JsonlFrameSink::new(FailingWriter, ProtocolLimits::default());
        assert!(matches!(
            sink.publish(&patch("<svg/>")),
            Err(TransportError::Io(_))
        ));
    }
}
